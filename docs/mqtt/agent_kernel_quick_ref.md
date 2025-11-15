# Agent-Kernel Communication - Quick Reference

## 1. Answer to Your 6 Questions

### 1. What protocol/technology is used?
**MQTT 3.1.1** (Message Queuing Telemetry Transport)
- Library: `rumqttc` (async Rust client)
- Broker: Mosquitto on `localhost:1883`
- QoS: `AtLeastOnce` (reliability)

### 2. What files implement the agent-side communication?
**Agent (symbion-agent-host/src/main.rs)**
- Lines 144-232: MQTT connection setup
- Lines 253-302: Main event loop (tokio::select!)
- Lines 305-329: Registration messages
- Lines 332-360: Heartbeat publishing
- Lines 363-462: Command reception & execution
- Lines 465-902: Command implementations

### 3. What files implement the kernel-side communication?
**Kernel (symbion-kernel/src/)**
- `mqtt.rs` (189 lines): MQTT subscriptions & message routing
- `agents.rs` (624 lines): Agent registry & command dispatch
- `http.rs`: HTTP endpoints (POST /agents/{id}/shutdown, etc.)

### 4. What message format is used?
**JSON** over MQTT, with 4 message types:
1. **Registration** - Agent announces itself
2. **Heartbeat** - Agent sends metrics every 30 seconds
3. **Command** - Kernel orders agent to do something
4. **Response** - Agent reports command result

### 5. What topics/channels are used?
```
symbion/agents/registration@v1   ← Agent announces
symbion/agents/heartbeat@v1      ← Agent sends metrics (30s interval)
symbion/agents/command@v1        ← Kernel commands agent
symbion/agents/response@v1       ← Agent responds
```

### 6. Are there heartbeats or health checks?
**Yes**, multiple layers:
- Agent heartbeat: Every 30 seconds
- Kernel monitoring: Every 60 seconds
- Timeout threshold: 5 minutes offline = marked offline

---

## 2. Message Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                    STARTUP & DISCOVERY                              │
└─────────────────────────────────────────────────────────────────────┘

  AGENT                      MQTT BROKER                    KERNEL
    |                            |                             |
    |--- Register @v1 ---------> |                             |
    |  (MAC, hostname, OS)       |--- Register @v1 ----------> |
    |                            |                             |
    |                            |                      [Store agent]
    |                            |                      [Set online]
    |
    |--- Subscribe to command@v1
    |

┌─────────────────────────────────────────────────────────────────────┐
│                   PERIODIC TELEMETRY (30s)                          │
└─────────────────────────────────────────────────────────────────────┘

    |                            |                             |
    |--- Heartbeat @v1 -------> |                             |
    | (CPU, RAM, disk, uptime)   |--- Heartbeat @v1 --------> |
    |                            |                      [Update metrics]
    |                            |                      [last_seen = now]
    |                            |
    | [wait 30 seconds]          |
    |                            |

┌─────────────────────────────────────────────────────────────────────┐
│                      COMMAND EXECUTION (ON-DEMAND)                  │
└─────────────────────────────────────────────────────────────────────┘

                                 |
                                 |<-- HTTP POST /agents/xxx/shutdown
                                 |    [App requests command]
                                 |
    |                            |                             |
    |<-- Command @v1 ---------- |                             |
    | {command_id, type, params}|                             |
    |                            |                             |
    | [execute command]          |                             |
    |  (fork process, wait)      |                             |
    |                            |                             |
    |--- Response @v1 --------> |                             |
    | {status, output, error}    |--- Response @v1 ----------> |
    |                            |                      [Store result]
    |                            |                      [Update status]
    |                            |                             |
    |                            |                      HTTP GET
    |                            |                      /agents/cmd_id/status
    |                            |                      <--- response

┌─────────────────────────────────────────────────────────────────────┐
│                    HEALTH MONITORING (1m interval)                  │
└─────────────────────────────────────────────────────────────────────┘

    KERNEL (background task every 60s):
    ├─ Check all agents
    ├─ For each agent:
    │  └─ if (now - last_seen > 5 minutes)
    │     ├─ Mark offline
    │     └─ Update registry
    └─ Save agents.json
```

---

## 3. JSON Message Examples

### Registration (Agent → Kernel on startup)
```json
{
  "agent_id": "a1b2c3d4e5f6",
  "hostname": "eridwyn-Salon",
  "os": "linux",
  "architecture": "x86_64",
  "capabilities": ["power_management", "process_control"],
  "network": {...},
  "version": "1.0.0",
  "timestamp": "2025-11-14T10:30:45Z"
}
```

### Heartbeat (Agent → Kernel every 30 seconds)
```json
{
  "agent_id": "a1b2c3d4e5f6",
  "status": "online",
  "system": {
    "uptime_seconds": 268800,
    "cpu": { "percent": 12.5, "load_avg": [0.8, 0.9, 1.1], "core_count": 4 },
    "memory": { "total_mb": 8192, "used_mb": 4096, "percent_used": 50.0 },
    "disk": [...]
  },
  "processes": { "total_count": 256, "top_cpu": [...], "top_memory": [...] },
  "services": [{"name": "ssh", "status": "active"}],
  "timestamp": "2025-11-14T10:30:45Z"
}
```

### Command (Kernel → Agent on-demand)
```json
{
  "command_id": "uuid-5678",
  "agent_id": "a1b2c3d4e5f6",
  "command_type": "shutdown",
  "parameters": null,
  "timeout_seconds": 30,
  "timestamp": "2025-11-14T10:31:00Z"
}
```

### Response (Agent → Kernel after command)
```json
{
  "command_id": "uuid-5678",
  "agent_id": "a1b2c3d4e5f6",
  "status": "success",
  "output": { "message": "Shutdown initiated" },
  "error": null,
  "execution_time_ms": 125,
  "timestamp": "2025-11-14T10:31:05Z"
}
```

---

## 4. Command Types

| Command | Platform | Action | Location |
|---------|----------|--------|----------|
| `shutdown` | Linux, Windows | System shutdown | main.rs:465-538 |
| `reboot` | Linux, Windows | System reboot | main.rs:541-613 |
| `hibernate` | Linux, Windows | Sleep/hibernation | main.rs:616-688 |
| `kill_process` | Linux, Windows | Kill by PID | main.rs:691-776 |
| `run_command` | Linux, Windows | Shell execution | main.rs:779-902 |
| `get_metrics` | Linux, Windows | Collect metrics | main.rs:905-931 |
| `list_processes` | Linux, Windows | List all procs | main.rs:934-958 |

---

## 5. Timing Intervals

| Component | Interval | Purpose |
|-----------|----------|---------|
| Agent Heartbeat | 30 seconds | Send telemetry |
| MQTT Keep-Alive | 15-30 seconds | Detect broker disconnect |
| Kernel Monitoring | 60 seconds | Check for offline agents |
| Offline Threshold | 5 minutes | Mark agent offline |
| Re-registration | 300 seconds (5 min) | Periodic re-sync |

---

## 6. File Structure Summary

### Agent Side
```
symbion-agent-host/src/
├── main.rs (1189 lines)
│   ├── MQTT client setup (144-232)
│   ├── Main event loop (253-302)
│   ├── Register (305-329)
│   ├── Heartbeat (332-360)
│   ├── Process command (363-462)
│   └── Execute commands (465-902)
├── config.rs (200+ lines)
│   └── MQTT broker config
└── metrics/
    └── System metrics collection
```

### Kernel Side
```
symbion-kernel/src/
├── mqtt.rs (189 lines)
│   ├── create_mqtt_client() (21-42)
│   └── spawn_mqtt_listener() (44-188)
├── agents.rs (624 lines)
│   ├── Agent struct definition
│   ├── handle_agent_registration() (306-342)
│   ├── handle_agent_heartbeat() (345-368)
│   ├── send_command() (386-428)
│   ├── handle_agent_response() (486-518)
│   └── Agent monitoring (587-621)
└── http.rs
    └── REST endpoints (POST /agents/{id}/shutdown, etc.)
```

---

## 7. Key Code Patterns

### Agent Publishes Heartbeat
```rust
// Main loop (every 30 seconds)
self.mqtt_client
    .publish("symbion/agents/heartbeat@v1", QoS::AtLeastOnce, false, payload)
    .await?
```

### Kernel Sends Command
```rust
// agents.rs:386-428
let topic = "symbion/agents/command@v1";
let payload = serde_json::to_string(&command)?;
mqtt_client.publish(topic, rumqttc::QoS::AtLeastOnce, false, payload).await?
```

### Kernel Receives Messages
```rust
// mqtt.rs:56-78 (subscriptions)
client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce).await?
client.subscribe("symbion/agents/response@v1", QoS::AtLeastOnce).await?
```

### Agent Receives Commands
```rust
// main.rs:183 (subscribes once on startup)
mqtt_client_for_loop.subscribe("symbion/agents/command@v1", QoS::AtLeastOnce).await?

// main.rs:191-208 (in event loop)
Ok(Event::Incoming(Incoming::Publish(publish))) => {
    if publish.topic == "symbion/agents/command@v1" {
        // Forward to main loop via mpsc channel
    }
}
```

---

## 8. Execution Flow for "Shutdown Command"

```
1. User clicks "Shutdown" on PWA Dashboard
   └─> HTTP POST /agents/a1b2c3d4e5f6/shutdown

2. Kernel HTTP handler (http.rs)
   └─> app.agents.send_command("a1b2c3d4e5f6", "shutdown", None)

3. Kernel agents.rs:send_command()
   ├─ Create command_id = "uuid-5678"
   ├─ Store pending command (status: Sent)
   └─> Publish to "symbion/agents/command@v1"

4. MQTT Broker routes message

5. Agent main.rs:161-216 (event loop)
   └─> Receive publish event
   └─> Forward via command_receiver channel

6. Agent main.rs:286-302 (main select loop)
   └─> command_receiver.recv()
   └─> process_command(cmd)

7. Agent main.rs:363-462 (process_command)
   ├─ Parse JSON
   ├─ Verify agent_id matches
   └─> execute_shutdown()

8. Agent main.rs:465-538 (execute_shutdown)
   └─> Call OS shutdown command
   └─> Record status = "success"

9. Agent main.rs:407-461 (build response)
   └─> Publish to "symbion/agents/response@v1"

10. MQTT Broker routes message

11. Kernel mqtt.rs:165-177 (listener)
    └─> Receive response on "symbion/agents/response@v1"
    └─> handle_agent_response()

12. Kernel agents.rs:486-518 (handle_agent_response)
    └─> Find pending command "uuid-5678"
    └─> Update status = "success"
    └─> Store output

13. User polls HTTP GET /agents/uuid-5678/status
    └─> Returns: status=success, output={message: "..."}
```

---

## 9. Connection Resilience

### Agent Reconnection Logic
```
Lost connection → 5 second backoff → Retry → ConnAck → Re-subscribe
                   (main.rs:211-215)          (main.rs:182-189)
```

### Kernel Offline Detection
```
Every 60 seconds:
  For each agent:
    if (now - last_seen > 5 minutes) AND status == "online"
      → Mark offline
      → Save registry
```

---

## 10. Security Model

**Authentication**: MAC-based agent ID
- Agent identified by MAC address without colons
- Example: `a1b2c3d4e5f6`

**Command Filtering**: Agent only executes commands for its own ID
- Code: main.rs:371-377
- All commands include target `agent_id`

**Command Whitelist**: Only safe shell commands allowed
- Code: main.rs:796-805
- Blocked: rm, dd, mkfs, dangerous commands

**Output Sanitization**: Remove dangerous characters
- Code: main.rs:820-838
- Removes ANSI codes and control characters

---

## 11. Debugging Commands

```bash
# Check all agents via HTTP
curl http://localhost:8080/agents

# Send shutdown to specific agent
curl -X POST http://localhost:8080/agents/a1b2c3d4e5f6/shutdown

# Check command status
curl http://localhost:8080/agents/a1b2c3d4e5f6/commands/cmd-uuid

# Get MQTT logs (if running locally)
mosquitto_sub -t 'symbion/#' -v

# Check agent health
curl http://localhost:8080/system/health
```

---

## 12. File Locations

**Agent Configuration**:
- Linux: `~/.config/symbion-agent/config.toml`
- Windows: `%APPDATA%\symbion-agent\config.toml`

**Kernel Agent Registry**:
- Default: `~/.symbion/agents.json`
- Configurable via environment

---

## 13. Common Scenarios

### Scenario 1: Agent First Startup
1. Agent connects to MQTT
2. Publishes registration (MAC, hostname, OS, capabilities)
3. Kernel receives, stores in registry, marks online
4. Starts sending heartbeats every 30 seconds

### Scenario 2: Network Disconnect
1. Agent loses MQTT connection
2. Broker fires MQTT disconnect event
3. Agent retries every 5 seconds (exponential backoff)
4. Kernel monitoring detects missing heartbeat after 5 minutes
5. Marks agent offline (no deletion)

### Scenario 3: Command Execution
1. Kernel sends command with 30s timeout
2. Agent executes, measures time
3. Publishes response (success/error/truncated)
4. Kernel receives, stores in pending commands map
5. Response expires after 5 minutes in kernel

### Scenario 4: Large Output
1. Agent executes command
2. Output > 7000 characters
3. Truncates with notice: `[OUTPUT TRUNCATED...]`
4. Publishes truncated response
5. Kernel stores truncated output

---

## Quick Stats

| Metric | Value |
|--------|-------|
| Total MQTT Topics | 5 (agents namespace) |
| Message Types | 4 (registration, heartbeat, command, response) |
| Max Output Size | 7000 characters |
| Command Timeout | 30 seconds |
| Heartbeat Interval | 30 seconds |
| Offline Threshold | 5 minutes |
| Monitoring Interval | 60 seconds |
| QoS Level | 1 (AtLeastOnce) |
| Command Types | 7 |
| Multi-OS Support | Linux, Windows |

