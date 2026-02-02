// Persistence JSONL append-only
// Spec: PR3 P0 v3.1 REFINED

use crate::decision::{DecisionRecord, Clock};
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::{Context, Result};

/// Mode de synchronisation fsync
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsyncMode {
    Full,      // fsync après chaque write (max durabilité)
    Batched,   // fsync périodique (compromis perf/durabilité)
    None,      // async OS (max performance, risque perte données crash)
}

/// Gestionnaire de persistance JSONL
pub struct PersistenceManager {
    file_path: PathBuf,
    file: Arc<RwLock<File>>,
    fsync_mode: FsyncMode,
    clock: Arc<dyn Clock>,
}

impl PersistenceManager {
    /// Créer un nouveau gestionnaire de persistance
    pub fn new<P: AsRef<Path>>(
        file_path: P,
        fsync_mode: FsyncMode,
        clock: Arc<dyn Clock>,
    ) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Créer répertoire parent si nécessaire
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }

        // Ouvrir fichier en mode append
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .with_context(|| format!("Failed to open persistence file: {:?}", file_path))?;

        Ok(Self {
            file_path,
            file: Arc::new(RwLock::new(file)),
            fsync_mode,
            clock,
        })
    }

    /// Persister une décision
    pub fn persist(&self, record: &DecisionRecord) -> Result<()> {
        let mut file = self.file.write();

        // Sérialiser en JSON + newline
        let json = serde_json::to_string(record)
            .context("Failed to serialize DecisionRecord")?;
        writeln!(file, "{}", json)
            .context("Failed to write record to file")?;

        // Fsync selon mode
        match self.fsync_mode {
            FsyncMode::Full => {
                file.sync_all()
                    .context("Failed to fsync after write")?;
            }
            FsyncMode::Batched | FsyncMode::None => {
                // Pas de fsync immédiat
            }
        }

        Ok(())
    }

    /// Forcer un fsync manuel (utile pour mode Batched)
    pub fn sync(&self) -> Result<()> {
        let file = self.file.write();
        file.sync_all()
            .context("Failed to fsync file")?;
        Ok(())
    }

    /// Lire toutes les décisions depuis le début
    pub fn read_all(&self) -> Result<Vec<DecisionRecord>> {
        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open file for reading: {:?}", self.file_path))?;

        let reader = BufReader::new(file);
        let mut records = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result
                .with_context(|| format!("Failed to read line {}", line_num + 1))?;

            if line.trim().is_empty() {
                continue; // Ignorer lignes vides
            }

            let record: DecisionRecord = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse JSON at line {}", line_num + 1))?;

            records.push(record);
        }

        Ok(records)
    }

    /// Lire les N dernières décisions
    pub fn read_last(&self, n: usize) -> Result<Vec<DecisionRecord>> {
        let all = self.read_all()?;
        let start = all.len().saturating_sub(n);
        Ok(all[start..].to_vec())
    }

    /// Lire les décisions depuis un timestamp
    pub fn read_since(&self, since: time::OffsetDateTime) -> Result<Vec<DecisionRecord>> {
        let all = self.read_all()?;
        Ok(all.into_iter()
            .filter(|r| r.timestamp >= since)
            .collect())
    }

    /// Statistiques basiques
    pub fn stats(&self) -> Result<PersistenceStats> {
        let records = self.read_all()?;

        let mut stats = PersistenceStats {
            total_decisions: records.len(),
            approved: 0,
            blocked: 0,
            require_validation: 0,
            dry_run: 0,
        };

        for record in records {
            match record.outcome {
                crate::decision::DecisionOutcome::Approved { .. } => stats.approved += 1,
                crate::decision::DecisionOutcome::Blocked { .. } => stats.blocked += 1,
                crate::decision::DecisionOutcome::RequireValidation { .. } => stats.require_validation += 1,
                crate::decision::DecisionOutcome::DryRun { .. } => stats.dry_run += 1,
            }
        }

        Ok(stats)
    }

    /// Obtenir le chemin du fichier
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Obtenir le mode fsync
    pub fn fsync_mode(&self) -> FsyncMode {
        self.fsync_mode
    }
}

/// Statistiques de persistance
#[derive(Debug, Clone)]
pub struct PersistenceStats {
    pub total_decisions: usize,
    pub approved: usize,
    pub blocked: usize,
    pub require_validation: usize,
    pub dry_run: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{
        Action, DecisionOutcome, DecisionRecord, ImpactLevel, SystemClock,
    };
    use time::macros::datetime;
    use uuid::Uuid;

    fn create_test_record() -> DecisionRecord {
        DecisionRecord {
            decision_id: Uuid::new_v4().to_string(),
            trace_id: "test-trace".to_string(),
            action_type: "test_action".to_string(),
            agent_id: "agent1".to_string(),
            impact_level: ImpactLevel::Low,
            outcome: DecisionOutcome::Approved {
                trust_score: 0.85,
                auto: true,
            },
            trust_score: None,
            timestamp: SystemClock.now_utc(),
            config_version: 1,
        }
    }

    #[test]
    fn test_persistence_write_read() {
        let temp_file = format!("/tmp/test-persistence-{}.jsonl", Uuid::new_v4());
        let clock = Arc::new(SystemClock);

        let manager = PersistenceManager::new(&temp_file, FsyncMode::Full, clock)
            .expect("Failed to create manager");

        // Écrire un record
        let record = create_test_record();
        manager.persist(&record).expect("Failed to persist");

        // Lire tous les records
        let records = manager.read_all().expect("Failed to read");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].decision_id, record.decision_id);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_persistence_multiple_records() {
        let temp_file = format!("/tmp/test-persistence-{}.jsonl", Uuid::new_v4());
        let clock = Arc::new(SystemClock);

        let manager = PersistenceManager::new(&temp_file, FsyncMode::None, clock)
            .expect("Failed to create manager");

        // Écrire 5 records
        for _ in 0..5 {
            let record = create_test_record();
            manager.persist(&record).expect("Failed to persist");
        }

        // Lire tous
        let records = manager.read_all().expect("Failed to read");
        assert_eq!(records.len(), 5);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_persistence_read_last() {
        let temp_file = format!("/tmp/test-persistence-{}.jsonl", Uuid::new_v4());
        let clock = Arc::new(SystemClock);

        let manager = PersistenceManager::new(&temp_file, FsyncMode::None, clock)
            .expect("Failed to create manager");

        // Écrire 10 records
        for _ in 0..10 {
            let record = create_test_record();
            manager.persist(&record).expect("Failed to persist");
        }

        // Lire les 3 derniers
        let last_3 = manager.read_last(3).expect("Failed to read last");
        assert_eq!(last_3.len(), 3);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_persistence_stats() {
        let temp_file = format!("/tmp/test-persistence-{}.jsonl", Uuid::new_v4());
        let clock = Arc::new(SystemClock);

        let manager = PersistenceManager::new(&temp_file, FsyncMode::None, clock)
            .expect("Failed to create manager");

        // Écrire 2 approved, 1 blocked
        for _ in 0..2 {
            let mut record = create_test_record();
            record.outcome = DecisionOutcome::Approved {
                trust_score: 0.9,
                auto: true,
            };
            manager.persist(&record).expect("Failed to persist");
        }

        let mut record = create_test_record();
        record.outcome = DecisionOutcome::Blocked {
            reasons: vec!["test".to_string()],
            explanation_codes: vec!["TEST".to_string()],
            categories: vec![],
        };
        manager.persist(&record).expect("Failed to persist");

        // Stats
        let stats = manager.stats().expect("Failed to get stats");
        assert_eq!(stats.total_decisions, 3);
        assert_eq!(stats.approved, 2);
        assert_eq!(stats.blocked, 1);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_persistence_read_since() {
        let temp_file = format!("/tmp/test-persistence-{}.jsonl", Uuid::new_v4());
        let clock = Arc::new(SystemClock);

        let manager = PersistenceManager::new(&temp_file, FsyncMode::None, clock)
            .expect("Failed to create manager");

        let now = SystemClock.now_utc();

        // Écrire des records avec timestamps différents
        let mut old_record = create_test_record();
        old_record.timestamp = now - time::Duration::hours(2);
        manager.persist(&old_record).expect("Failed to persist");

        let mut recent_record = create_test_record();
        recent_record.timestamp = now;
        manager.persist(&recent_record).expect("Failed to persist");

        // Lire depuis 1h
        let since = now - time::Duration::hours(1);
        let recent = manager.read_since(since).expect("Failed to read since");
        assert_eq!(recent.len(), 1);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }
}
