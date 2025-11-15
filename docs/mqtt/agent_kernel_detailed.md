# Symbion Agent-Kernel Communication Architecture

## Executive Summary

The Symbion ecosystem uses **MQTT (Message Queuing Telemetry Transport)** as the primary communication protocol between distributed agents and the central kernel. Communication is **asynchronous, event-driven, and contract-based** with a strict hierarchy: the Kernel sends commands via MQTT, and Agents respond with telemetry and command results.

---

## 1. Protocol & Technology

### Primary Technology: MQTT 3.1.1
- **Library**: `rumqttc` (Rust async MQTT client)
- **Broker**: Typically Mosquitto on `localhost:1883`
- **Quality of Service (QoS)**: `AtLeastOnce` (QoS 1) for reliability
- **Keep-Alive**: 15-30 seconds
- **Client IDs**: 
  - Kernel: `symbion-kernel-bridge` (publisher) and `symbion-kernel-listener` (subscriber)
  - Agents: `symbion-agent-{agent_id}` (derived from system MAC address)

### Secondary Technology: HTTP/REST
- **Kernel HTTP API** on port 8080 (TLS on 8443)
- Used by PWA dashboard to command agents indirectly
- HTTP endpoints call MQTT under the hood

---

## 2. File Structure

### Agent-Side Implementation
**Primary File**: `/home/eridwyn/RustroverProjects/NewSymbion/symbion-agent-host/src/main.rs` (1189 lines)

Agent communication flow:
1. **Lines 144-232**: `Agent::new_with_config()` - Initializes MQTT connection
2. **Lines 253-302**: `Agent::run()` - Main event loop with tokio::select!
3. **Lines 305-329**: `Agent::register()` - Registration message (lines 308-327)
4. **Lines 332-360**: `Agent::send_heartbeat()` - Heartbeat message (lines 339-347)
5. **Lines 363-462**: `Agent::process_command()` - Command reception and execution
6. **Lines 465-902**: Command executors (shutdown, reboot, hibernate, etc.)

### Kernel-Side Implementation
**Primary Files**:
- `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/mqtt.rs` (189 lines)
- `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/agents.rs` (624 lines)
- `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/http.rs` (endpoints)

Kernel communication flow:
1. **mqtt.rs:21-42**: `create_mqtt_client()` - Publisher setup
2. **mqtt.rs:44-188**: `spawn_mqtt_listener()` - Subscription and message handling
3. **agents.rs:306-342**: `handle_agent_registration()` - Process registration
4. **agents.rs:345-368**: `handle_agent_heartbeat()` - Process heartbeat
5. **agents.rs:386-428**: `send_command()` - Send command via MQTT
6. **agents.rs:486-518**: `handle_agent_response()` - Process command responses

---

## 3. MQTT Topics and Message Flow

### Topic Naming Convention
Format: `symbion/{namespace}/{event}@{version}`

Examples:
- `symbion/agents/registration@v1`
- `symbion/agents/heartbeat@v1`
- `symbion/agents/command@v1`
- `symbion/agents/response@v1`

### Complete Message Flow

```
AGENT                           KERNEL
 |                               |
 |--- [1] Registration ---------> |
 |     (symbion/agents/registration@v1)
 |
 |<-- Subscribed to command topic
 |
 |--- [2] Heartbeat (periodic) -> |
 |     (symbion/agents/heartbeat@v1, every 30 secs)
 |
 |<-- Receives command ---------- |
 |     (symbion/agents/command@v1)
 |
 | [execute command]
 |
 |--- [3] Response ------------> |
 |     (symbion/agents/response@v1)
 |
 |--- [2] Heartbeat (periodic) -> |
 |
```

### Topics Subscribed by Kernel
Located in `symbion-kernel/src/mqtt.rs:56-78`:

```rust
// Kernel subscriptions:
"symbion/hosts/heartbeat@v2"        // Legacy host heartbeats
"symbion/notes/response@v1"         // Plugin notes responses
"symbion/agents/registration@v1"    // Agent registration events
"symbion/agents/heartbeat@v1"       // Agent system metrics
"symbion/agents/response@v1"        // Command execution responses
```

### Topics Published by Agents
Located in `symbion-agent-host/src/main.rs:305-460`:

```rust
// Agent publishes to:
"symbion/agents/registration@v1"    // Initial connection (lines 323)
"symbion/agents/heartbeat@v1"       // Every 30 seconds (line 354)
"symbion/agents/response@v1"        // After command execution (line 442, 456)

// Agent subscribes to:
"symbion/agents/command@v1"         // Commands from kernel (line 183)
```

---

## 4. Message Formats (JSON)

### Message 1: Registration (Agent → Kernel)
**Topic**: `symbion/agents/registration@v1`
**Sent**: Once on startup + periodic re-registration every 5 minutes
**Source**: `symbion-agent-host/src/main.rs:61-71` and `main.rs:305-329`

```json
{
  "agent_id": "a1b2c3d4e5f6",          // MAC address without colons
  "hostname": "eridwyn-Salon",
  "os": "linux",
  "architecture": "x86_64",
  "capabilities": [
    "power_management",
    "process_control",
    "system_metrics",
    "command_execution"
  ],
  "network": {
    "primary_mac": "a1:b2:c3:d4:e5:f6",
    "interfaces": [
      {
        "name": "eth0",
        "mac": "a1:b2:c3:d4:e5:f6",
        "ip": "192.168.1.100",
        "type": "ethernet"
      }
    ]
  },
  "version": "1.0.0",
  "timestamp": "2025-11-14T10:30:45Z"
}
```

**Kernel Processing**: `symbion-kernel/src/agents.rs:306-342`
- Creates `Agent` struct with metadata
- Saves to JSON registry file
- Status: "online"

---

### Message 2: Heartbeat (Agent → Kernel)
**Topic**: `symbion/agents/heartbeat@v1`
**Frequency**: Every 30 seconds
**Source**: `symbion-agent-host/src/main.rs:332-360`

```json
{
  "agent_id": "a1b2c3d4e5f6",
  "status": "online",
  "system": {
    "uptime_seconds": 268800,
    "cpu": {
      "percent": 12.5,
      "load_avg": [0.8, 0.9, 1.1],
      "core_count": 4
    },
    "memory": {
      "total_mb": 8192,
      "used_mb": 4096,
      "available_mb": 4096,
      "percent_used": 50.0
    },
    "disk": [
      {
        "path": "/",
        "total_gb": 256.0,
        "used_gb": 128.0,
        "free_gb": 128.0,
        "percent_used": 50.0
      }
    ],
    "network": {
      "interfaces": [
        {
          "name": "eth0",
          "bytes_sent": 1000000,
          "bytes_recv": 2000000,
          "packets_sent": 10000,
          "packets_recv": 15000,
          "is_up": true
        }
      ]
    },
    "temperature": {
      "cpu_celsius": 45.0,
      "sensors": []
    }
  },
  "processes": {
    "total_count": 256,
    "running_count": 4,
    "top_cpu": [
      { "pid": 1234, "name": "firefox", "cpu_percent": 25.5, "memory_mb": 512.0 }
    ],
    "top_memory": [
      { "pid": 5678, "name": "systemd", "cpu_percent": 0.1, "memory_mb": 256.0 }
    ]
  },
  "services": [
    { "name": "ssh", "status": "active", "enabled": true },
    { "name": "mqtt", "status": "active", "enabled": true }
  ],
  "last_command": {
    "command_id": "uuid-1234",
    "command_type": "shutdown",
    "status": "success",
    "timestamp": "2025-11-14T10:29:45Z"
  },
  "timestamp": "2025-11-14T10:30:45Z"
}
```

**Kernel Processing**: `symbion-kernel/src/agents.rs:345-368`
- Updates agent status in memory
- Sets `last_heartbeat` timestamp
- Updates system metrics
- Triggers agent monitoring (timeout detection)

---

### Message 3: Command (Kernel → Agent)
**Topic**: `symbion/agents/command@v1`
**Sent**: On-demand via HTTP API or internal Kernel request
**Source**: `symbion-kernel/src/agents.rs:386-428`

```json
{
  "command_id": "cmd-uuid-5678",
  "agent_id": "a1b2c3d4e5f6",
  "command_type": "shutdown",
  "parameters": null,
  "timeout_seconds": 30,
  "timestamp": "2025-11-14T10:31:00Z"
}
```

**Command Types**: `symbion-agent-host/src/main.rs:382-397`
- `shutdown` - System shutdown (lines 465-538)
- `reboot` - System reboot (lines 541-613)
- `hibernate` - Hibernation (lines 616-688)
- `kill_process` - Kill process by PID (lines 691-776)
- `run_command` - Execute shell command (lines 779-902)
- `get_metrics` - Collect current metrics (lines 905-931)
- `list_processes` - List all processes (lines 934-958)

**Agent Processing**: `symbion-agent-host/src/main.rs:363-462`
1. Parse incoming command JSON
2. Filter: only execute if `command_id` matches this agent
3. Execute based on command type
4. Build response
5. Publish response to `symbion/agents/response@v1`

---

### Message 4: Response (Agent → Kernel)
**Topic**: `symbion/agents/response@v1`
**Sent**: After command execution
**Source**: `symbion-agent-host/src/main.rs:407-461`

```json
{
  "command_id": "cmd-uuid-5678",
  "agent_id": "a1b2c3d4e5f6",
  "status": "success",
  "output": {
    "message": "Shutdown initiated"
  },
  "error": null,
  "execution_time_ms": 125,
  "timestamp": "2025-11-14T10:31:05Z"
}
```

**Response Status Values**:
- `"success"` - Command executed successfully
- `"error"` - Command failed with error
- `"in_progress"` - Command still running
- `"cancelled"` - Command was cancelled

**Kernel Processing**: `symbion-kernel/src/agents.rs:486-518`
- Updates pending command status
- Stores output/error data
- Updates command state machine

**Output Truncation**: `symbion-agent-host/src/main.rs:419-449`
- Maximum output size: 7000 characters
- Larger outputs are truncated with notice: `[OUTPUT TRUNCATED - Content was too large for MQTT transport]`

---

## 5. Health Checks & Timeouts

### Agent Heartbeat Interval
- **Frequency**: Every 30 seconds (configurable)
- **Keep-Alive**: 30 second MQTT keep-alive
- **Location**: `symbion-agent-host/src/main.rs:263-264`

```rust
let mut heartbeat_timer = interval(Duration::from_secs(self.config.heartbeat_interval_secs));
```

### Kernel Monitoring
- **Check Frequency**: Every 60 seconds (1 minute)
- **Offline Threshold**: Configurable timeout (default: 5 minutes)
- **Location**: `symbion-kernel/src/agents.rs:587-621`

```rust
// Agent monitoring loop
let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

// Timeout threshold calculation
let timeout_threshold = now - time::Duration::minutes(timeout_minutes);
```

### Detection Logic
- Agents marked "online" if last_seen ≤ timeout threshold
- Agents marked "offline" if last_seen > timeout threshold
- **Timeout Detection**: `symbion-kernel/src/agents.rs:604-607`

```rust
if agent.status.status == "online" && agent.last_seen < timeout_threshold {
    agents_to_mark_offline.push(agent_id.clone());
}
```

### Kernel Health Tracking
- **Module**: `symbion-kernel/src/health.rs`
- Tracks MQTT connection status
- Records message activity
- Called on each message: `health_tracker.record_mqtt_message()`
- MQTT marked "connected" after successful subscriptions

---

## 6. Command Execution Flow (Detailed)

### HTTP → MQTT Command Chain

**Step 1: HTTP Endpoint (Kernel)**
Location: `symbion-kernel/src/http.rs`
```rust
// POST /agents/{id}/shutdown
async fn agent_shutdown_endpoint(...) {
    match app.agents.send_command(&id, "shutdown", None).await { ... }
}
```

**Step 2: Send Command via MQTT (Kernel)**
Location: `symbion-kernel/src/agents.rs:386-428`
```rust
pub async fn send_command(&self, agent_id: &str, command_type: &str, parameters: Option<serde_json::Value>) -> Result<String> {
    // Create command ID
    let command_id = Uuid::new_v4().to_string();
    
    // Create pending command tracker
    let pending_command = PendingCommand {
        command_id: command_id.clone(),
        agent_id: agent_id.to_string(),
        command_type: command_type.to_string(),
        parameters: parameters.clone(),
        timestamp: OffsetDateTime::now_utc(),
        timeout: Duration::from_secs(30),
        status: CommandStatus::Sent,
        output: None,
        error: None,
    };
    
    // Publish to MQTT
    let topic = "symbion/agents/command@v1";
    let payload = serde_json::to_string(&command)?;
    mqtt_client.publish(topic, rumqttc::QoS::AtLeastOnce, false, payload).await?;
    
    // Return command_id for tracking
    Ok(command_id)
}
```

**Step 3: Receive & Execute Command (Agent)**
Location: `symbion-agent-host/src/main.rs:286-302`
```rust
// In main loop with tokio::select!
command = self.command_receiver.recv() => {
    match command {
        Some(cmd) => {
            if let Err(e) = self.process_command(cmd).await {
                error!("Failed to process command: {}", e);
            }
        }
        ...
    }
}
```

**Step 4: Send Response (Agent)**
Location: `symbion-agent-host/src/main.rs:407-461`
```rust
let response = CommandResponse {
    command_id: incoming.command_id,
    agent_id: self.system_info.agent_id.clone(),
    status,
    output: data,
    error,
    execution_time_ms: execution_time,
    timestamp: Utc::now(),
};

self.mqtt_client
    .publish("symbion/agents/response@v1", QoS::AtLeastOnce, false, payload)
    .await?;
```

**Step 5: Receive Response (Kernel)**
Location: `symbion-kernel/src/mqtt.rs:165-177`
```rust
} else if p.topic == "symbion/agents/response@v1" {
    if let Some(ref agent_registry) = agents {
        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
            match serde_json::from_str::<AgentResponse>(&txt) {
                Ok(response) => {
                    if let Err(e) = agent_registry.handle_agent_response(response).await {
                        eprintln!("[kernel] failed to handle agent response: {}", e);
                    }
                }
                ...
            }
        }
    }
}
```

**Step 6: Query Status (HTTP)**
```bash
GET /agents/{command_id}/status
```
Returns `PendingCommand` with current state

---

## 7. Contract System

### Contract Registry
- **Module**: `symbion-kernel/src/contracts.rs`
- **Purpose**: Schema validation and versioning
- **Location**: `contracts/mqtt/` directory

### Contract Name Format
- Topic: `symbion/agents/command@v1`
- Contract name: `agents.command@v1`
- Extraction: `symbion/{namespace}/{event}@{version}` → `{namespace}.{event}@{version}`

### Available Contracts
Located in CLAUDE.md context:
- `agents.registration@v1` - Agent registration
- `agents.heartbeat@v1` - Agent telemetry
- `agents.command@v1` - Kernel commands
- `agents.response@v1` - Command responses
- `kernel.health@v1` - Kernel health metrics
- `notes.command@v1` & `notes.response@v1` - Notes plugin

---

## 8. Error Handling & Resilience

### Connection Recovery (Agent)
Location: `symbion-agent-host/src/main.rs:179-189`
```rust
Ok(Event::Incoming(Incoming::ConnAck(_))) => {
    // Reconnected - resubscribe to command topic
    info!("🔄 MQTT connected/reconnected - subscribing to command topic...");
    if let Err(e) = mqtt_client_for_loop.subscribe("symbion/agents/command@v1", QoS::AtLeastOnce).await {
        error!("Failed to subscribe to command topic: {}", e);
        is_subscribed = false;
    } else {
        info!("✅ Subscribed to symbion/agents/command@v1");
        is_subscribed = true;
    }
}
```

### Command Timeout
- **Duration**: 30 seconds per command
- **Location**: `symbion-kernel/src/agents.rs:394, 406`
- **Cleanup**: `cleanup_old_commands()` removes old commands after 5+ minutes

### Stale Agent Cleanup
Location: `symbion-kernel/src/agents.rs:561-584`
```rust
pub async fn cleanup_stale_agents(&self, max_age_hours: i64) -> Result<()> {
    let cutoff = OffsetDateTime::now_utc() - time::Duration::hours(max_age_hours);
    // Remove agents not seen for max_age_hours
}
```

### Broker Reconnection (Agent)
Location: `symbion-agent-host/src/main.rs:211-215`
```rust
Err(e) => {
    error!("MQTT connection error: {}", e);
    is_subscribed = false;
    tokio::time::sleep(Duration::from_secs(5)).await;  // 5 second retry backoff
}
```

---

## 9. Security Considerations

### Authentication
- No JWT authentication on MQTT level (broker assumed secured on LAN)
- Agent identification via MAC address (`agent_id`)
- Command filtering: Agent only executes commands for its own `agent_id`
  - Location: `symbion-agent-host/src/main.rs:371-377`

### Command Whitelisting
Location: `symbion-agent-host/src/main.rs:796-805`
```rust
let safe_commands = [
    "dir", "ls", "whoami", "hostname", "date", "uptime", "ps", 
    "tasklist", "shutdown", "echo", "systemctl", "cat", "tail", 
    "head", "pwd", "id", "ipconfig", "ifconfig", "powershell", 
    "get-process", "netstat", "ping", "tracert", "nslookup"
];
let is_safe = safe_commands.iter().any(|&safe_cmd| command.starts_with(safe_cmd));
```

### Output Sanitization
- ANSI escape codes stripped from command output
- Control characters removed to prevent JSON corruption
  - Location: `symbion-agent-host/src/main.rs:820-838`

---

## 10. Persistence & Synchronization

### Agent Registry Persistence
- **File**: `~/.symbion/agents.json` (configurable)
- **Format**: JSON serialization of `HashMap<String, Agent>`
- **Load**: On kernel startup via `load_agents()`
- **Save**: After registration + periodic agent monitoring updates
  - Location: `symbion-kernel/src/agents.rs:276-304`

### Periodic Saving
- **Frequency**: When agent state changes (registration, heartbeat, timeout)
- **Lock Strategy**: Minimal lock duration - snapshot before I/O
  - Location: `symbion-kernel/src/agents.rs:293-304`

```rust
pub async fn save_agents(&self) -> Result<()> {
    // Clone data snapshot BEFORE I/O pour minimiser durée du lock
    let agents_snapshot = {
        let agents_map = self.agents.read().await;
        agents_map.clone()
    }; // Libère le read lock immédiatement

    // Sérialisation et I/O SANS tenir de lock
    let content = serde_json::to_string_pretty(&agents_snapshot)?;
    tokio::fs::write(&self.data_file, content).await?;
    Ok(())
}
```

---

## 11. Performance Characteristics

### MQTT Message Throughput
- **Heartbeat Payload**: ~2-3KB per agent (varies with metrics)
- **Frequency**: 1 message per agent every 30 seconds
- **Network Impact**: Minimal on LAN (negligible overhead)

### Memory Footprint
- **Agent Registry**: O(n) where n = number of agents
- **Pending Commands**: O(m) where m = concurrent commands
- **Typical**: <10MB for 10 agents with 50 pending commands

### Latency
- **Command→Response**: 100-500ms typical
- **Registration**: ~50ms
- **Heartbeat Processing**: <100ms kernel-side

---

## 12. Multi-OS Support

### Linux Agent
- **Shutdown**: `sudo shutdown -h +1`
- **Reboot**: `sudo reboot`
- **Hibernate**: `systemctl hibernate`
- **Kill Process**: `kill -9 {pid}`
- **Shell Command**: `sh -c {command}`

### Windows Agent
- **Shutdown**: `shutdown /s /t 0 /f` (immediate force)
- **Reboot**: `shutdown /r /t 5 /c "..."`
- **Hibernate**: `rundll32.exe powrprof.dll,SetSuspendState Hibernate`
- **Kill Process**: `taskkill /PID {pid} /F`
- **Shell Command**: `cmd /C {command}`

Location: `symbion-agent-host/src/main.rs:465-902`

---

## 13. Integration with Dashboard

### Dashboard to Agent Command Chain
1. **PWA Dashboard** sends HTTP POST to kernel
2. **Kernel HTTP Handler** calls `app.agents.send_command()`
3. **Kernel** publishes MQTT message
4. **Agent** receives and executes
5. **Agent** publishes response MQTT message
6. **Kernel** receives response
7. **Dashboard** polls HTTP `GET /agents/{command_id}/status`

### WebSocket Dashboard Events
- Kernel publishes agent updates via `DashboardEventPublisher`
- Topic: `symbion/dashboard/agents@v1` (internal)
- Real-time agent list updates to PWA

Location: `symbion-kernel/src/mqtt.rs:151-158`
```rust
// Publier la liste des agents sur le dashboard topic
if let Some(ref dash_events) = dashboard_events {
    let agents_map = agent_registry.list_agents().await;
    let agents_list: Vec<crate::agents::Agent> = agents_map.values().cloned().collect();
    if let Err(e) = dash_events.publish_agents_update(&agents_list).await {
        eprintln!("[kernel] failed to publish agents update to dashboard: {}", e);
    }
}
```

---

## 14. Configuration

### Kernel MQTT Config
Location: `symbion-kernel/src/config.rs`
```rust
pub struct MqttConf {
    pub host: String,       // Default: "localhost"
    pub port: u16,          // Default: 1883
}
```

### Agent MQTT Config
Location: `symbion-agent-host/src/config.rs:23-28`
```rust
pub struct MqttConfig {
    pub broker_host: String,           // Loaded from config.toml
    pub broker_port: u16,              // Default: 1883
    pub client_id: Option<String>,     // Auto-generated if None
    pub keep_alive_secs: u16,          // Default: 60
}
```

Stored at:
- **Linux**: `~/.config/symbion-agent/config.toml`
- **Windows**: `%APPDATA%/symbion-agent/config.toml`

---

## 15. Debugging & Monitoring

### Logging
- **Agent**: `tracing` crate with info/error/debug levels
- **Kernel**: `eprintln!()` for errors, `println!()` for info
- **Typical output**:
  ```
  [kernel] MQTT connected and subscriptions active
  [agents] registered agent a1b2c3d4e5f6 (eridwyn-Salon)
  [agents] updating heartbeat for agent a1b2c3d4e5f6
  [kernel] MQTT connected and subscriptions active
  ```

### Health Monitoring
- **Kernel Health Check**: `/system/health` endpoint
- **Agent Monitoring**: Periodic scan for offline agents
- **Health Tracker**: Tracks MQTT status, message counts, connectivity

### Curl Examples

**Get all agents**:
```bash
curl http://localhost:8080/agents
```

**Send shutdown command**:
```bash
curl -X POST http://localhost:8080/agents/a1b2c3d4e5f6/shutdown
```

**Get command status**:
```bash
curl http://localhost:8080/agents/a1b2c3d4e5f6/commands/cmd-uuid-5678
```

**List processes**:
```bash
curl -X POST http://localhost:8080/agents/a1b2c3d4e5f6/processes
```

---

## Summary Table

| Aspect | Details |
|--------|---------|
| **Protocol** | MQTT 3.1.1 (rumqttc) |
| **Broker** | Mosquitto on localhost:1883 |
| **QoS** | AtLeastOnce (1) |
| **Heartbeat** | Every 30 seconds |
| **Timeout** | 5 minutes offline threshold |
| **Command Types** | 7 types (shutdown, reboot, etc.) |
| **Response Time** | 100-500ms typical |
| **Max Output** | 7000 chars (truncated) |
| **Persistence** | JSON file (~/.symbion/agents.json) |
| **Security** | MAC-based agent ID + command whitelist |
| **Multi-OS** | Linux & Windows support |

---

## Key Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `symbion-kernel/src/mqtt.rs` | 189 | MQTT listener & subscriptions |
| `symbion-kernel/src/agents.rs` | 624 | Agent registry & command sender |
| `symbion-kernel/src/http.rs` | ~2000 | HTTP endpoints calling agents.send_command() |
| `symbion-agent-host/src/main.rs` | 1189 | Agent main loop & MQTT publisher |
| `symbion-agent-host/src/config.rs` | 200+ | Agent config management |
| `symbion-kernel/src/contracts.rs` | 158 | Contract validation system |

