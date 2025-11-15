# Symbion Agent-Kernel Communication - Documentation Index

## Documentation Files

This exploration created two comprehensive documents describing how agents communicate with the Symbion kernel:

### 1. **AGENT_KERNEL_COMMUNICATION.md** (22 KB, 751 lines)
**For**: Detailed technical understanding and implementation reference

**Contents**:
- Section 1: Protocol & Technology (MQTT 3.1.1, rumqttc library)
- Section 2: File Structure (agent-side and kernel-side files)
- Section 3: MQTT Topics & Message Flow (visual diagrams)
- Section 4: Message Formats (complete JSON examples for all 4 message types)
- Section 5: Health Checks & Timeouts (30s heartbeat, 5m offline threshold)
- Section 6: Command Execution Flow (detailed HTTP→MQTT chain)
- Section 7: Contract System (schema validation)
- Section 8: Error Handling & Resilience
- Section 9: Security Considerations
- Section 10: Persistence & Synchronization
- Section 11: Performance Characteristics
- Section 12: Multi-OS Support (Linux & Windows)
- Section 13: Dashboard Integration
- Section 14: Configuration
- Section 15: Debugging & Monitoring

**Use when**: You need to understand the complete architecture, debug issues, or plan extensions.

---

### 2. **AGENT_KERNEL_COMMUNICATION_QUICK_REFERENCE.md** (15 KB, 452 lines)
**For**: Quick lookup and implementation guidance

**Contents**:
- Direct answers to your 6 questions
- Message flow ASCII diagrams
- JSON examples for each message type
- Command types table (7 types listed)
- Timing intervals table
- File structure summary
- Key code patterns
- Full execution flow walkthrough
- Connection resilience patterns
- Security model summary
- Debugging commands
- Common scenarios (startup, disconnect, execution, large output)
- Quick statistics table

**Use when**: You need quick answers, code snippets, or implementation examples.

---

## Quick Answers to Your 6 Questions

### 1. What protocol/technology is used?
**MQTT 3.1.1** using the `rumqttc` Rust async library
- Broker: Mosquitto on localhost:1883
- Quality of Service: AtLeastOnce (QoS 1) for reliability

**See**: AGENT_KERNEL_COMMUNICATION.md Section 1

---

### 2. What files implement agent-side communication?
**Single primary file**: `symbion-agent-host/src/main.rs` (1189 lines)

Breakdown by functionality:
- **Lines 144-232**: MQTT client initialization and connection setup
- **Lines 253-302**: Main event loop (tokio::select! pattern)
- **Lines 305-329**: Agent registration message publishing
- **Lines 332-360**: Heartbeat message publishing (every 30 seconds)
- **Lines 363-462**: Command message reception and processing
- **Lines 465-902**: Command execution implementations (7 types)

Supporting files:
- `config.rs`: MQTT broker configuration management
- `metrics/mod.rs`: System metrics collection
- `discovery.rs`: Network interface discovery

**See**: AGENT_KERNEL_COMMUNICATION.md Section 2, QUICK_REFERENCE.md Section 6

---

### 3. What files implement kernel-side communication?
**Three primary files**:

1. **`symbion-kernel/src/mqtt.rs`** (189 lines)
   - Lines 21-42: `create_mqtt_client()` - Publisher setup
   - Lines 44-188: `spawn_mqtt_listener()` - Subscription and message routing
   - Subscribes to 5 topics and dispatches messages to appropriate handlers

2. **`symbion-kernel/src/agents.rs`** (624 lines)
   - Agent struct definitions with network metadata
   - `handle_agent_registration()` (306-342): Process agent signup
   - `handle_agent_heartbeat()` (345-368): Update agent metrics
   - `send_command()` (386-428): Dispatch commands via MQTT
   - `handle_agent_response()` (486-518): Process command results
   - `start_agent_monitoring()` (587-621): Periodic offline detection

3. **`symbion-kernel/src/http.rs`** (~2000 lines)
   - HTTP endpoints that call `agents.send_command()`
   - Examples: POST /agents/{id}/shutdown, /agents/{id}/reboot, etc.

Supporting files:
- `contracts.rs`: Message schema validation and versioning
- `health.rs`: MQTT connectivity tracking

**See**: AGENT_KERNEL_COMMUNICATION.md Section 2, QUICK_REFERENCE.md Section 6

---

### 4. What message format is used?
**JSON** over MQTT, with 4 distinct message types:

1. **Registration Message** (agent → kernel)
   - Topic: `symbion/agents/registration@v1`
   - Sent: Once on startup, periodic re-registration every 5 minutes
   - Content: MAC address, hostname, OS, architecture, capabilities, network interfaces
   - JSON size: ~1-2 KB

2. **Heartbeat Message** (agent → kernel)
   - Topic: `symbion/agents/heartbeat@v1`
   - Frequency: Every 30 seconds
   - Content: System metrics (CPU, RAM, disk, uptime), processes, services, last command
   - JSON size: ~2-3 KB

3. **Command Message** (kernel → agent)
   - Topic: `symbion/agents/command@v1`
   - Sent: On-demand when user clicks action on dashboard
   - Content: command_id, agent_id, command_type, parameters, timeout
   - JSON size: ~0.5-1 KB

4. **Response Message** (agent → kernel)
   - Topic: `symbion/agents/response@v1`
   - Sent: After command execution
   - Content: command_id, agent_id, status, output, error, execution_time_ms
   - JSON size: Variable (max 7000 chars truncated if larger)

**See**: AGENT_KERNEL_COMMUNICATION.md Section 4, QUICK_REFERENCE.md Section 3

---

### 5. What topics/channels are used?
**4 main MQTT topics** in the `symbion/agents/` namespace:

```
symbion/agents/registration@v1   ← Agent announces itself (startup)
symbion/agents/heartbeat@v1      ← Agent sends metrics (every 30 seconds)
symbion/agents/command@v1        ← Kernel sends commands (on-demand)
symbion/agents/response@v1       ← Agent reports command results (immediate)
```

Additional topics subscribed by kernel:
- `symbion/hosts/heartbeat@v2` - Legacy host heartbeats
- `symbion/notes/response@v1` - Plugin responses

**Topic Format**:
- Pattern: `symbion/{namespace}/{event}@{version}`
- Example: `symbion/agents/heartbeat@v1`
- Contract Name: Derived from topic, e.g., `agents.heartbeat@v1`

**See**: AGENT_KERNEL_COMMUNICATION.md Section 3, QUICK_REFERENCE.md Section 5

---

### 6. Are there heartbeats or health checks?
**Yes, multiple layers of health monitoring**:

**Agent-Side Heartbeat**:
- Frequency: Every 30 seconds (configurable)
- Location: `symbion-agent-host/src/main.rs:263-264`
- Contains: Full system metrics (CPU, RAM, disk, processes, services)
- Ensures kernel knows agent is alive and healthy

**Kernel-Side Monitoring**:
- Check frequency: Every 60 seconds
- Location: `symbion-kernel/src/agents.rs:587-621`
- Logic: Check if `now - last_seen > 5 minutes`
- Action: Mark agent "offline" if heartbeat missing for 5 minutes

**MQTT Keep-Alive**:
- Duration: 15-30 seconds
- Purpose: Detect broker disconnection
- Recovery: 5 second backoff retry on connection failure

**Health Tracking Module**:
- Location: `symbion-kernel/src/health.rs`
- Tracks: MQTT connection status, message activity
- Called: On each received MQTT message

**Timing Summary**:
| Component | Interval | Purpose |
|-----------|----------|---------|
| Agent Heartbeat | 30 seconds | Send telemetry |
| Kernel Check | 60 seconds | Detect offline agents |
| Offline Mark | 5 minutes | Consider agent down |
| MQTT Keep-Alive | 15-30 seconds | Broker connection |

**See**: AGENT_KERNEL_COMMUNICATION.md Section 5, QUICK_REFERENCE.md Section 5

---

## Complete Message Flow

### Startup Sequence
```
1. Agent connects to MQTT broker
2. Agent publishes registration message
3. Kernel receives and stores agent metadata
4. Agent subscribes to command topic
```

### Periodic Operation (every 30 seconds)
```
5. Agent publishes heartbeat with metrics
6. Kernel receives and updates agent status
7. Kernel records last_seen timestamp
```

### Command Execution (on-demand)
```
8. User clicks action on PWA dashboard
9. HTTP POST to kernel
10. Kernel publishes command to MQTT
11. Agent receives and executes
12. Agent publishes response
13. Kernel stores result
14. User queries status via HTTP GET
```

### Health Monitoring (every 60 seconds)
```
15. Kernel checks all agents
16. If missing heartbeat > 5 minutes, mark offline
17. Save updated registry
```

**See**: AGENT_KERNEL_COMMUNICATION.md Section 6, QUICK_REFERENCE.md Section 8

---

## Key Statistics

| Metric | Value |
|--------|-------|
| Protocol | MQTT 3.1.1 |
| Library | rumqttc (Rust) |
| Heartbeat Interval | 30 seconds |
| Offline Threshold | 5 minutes |
| Monitoring Loop | 60 seconds |
| Command Timeout | 30 seconds |
| Max Output Size | 7000 characters (truncated) |
| Command Types | 7 (shutdown, reboot, hibernate, kill_process, run_command, get_metrics, list_processes) |
| MQTT Topics | 4 main channels |
| Message Types | 4 (registration, heartbeat, command, response) |
| Multi-OS | Linux & Windows |
| QoS Level | 1 (AtLeastOnce) |
| Typical Heartbeat Size | 2-3 KB JSON |

---

## File Locations

**Source Code**:
- Agent: `/home/eridwyn/RustroverProjects/NewSymbion/symbion-agent-host/src/main.rs`
- Kernel MQTT: `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/mqtt.rs`
- Kernel Agents: `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/agents.rs`
- Kernel HTTP: `/home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel/src/http.rs`

**Configuration**:
- Agent (Linux): `~/.config/symbion-agent/config.toml`
- Agent (Windows): `%APPDATA%\symbion-agent\config.toml`
- Agent Registry: `~/.symbion/agents.json`

**Documentation**:
- This file: `AGENT_KERNEL_COMMUNICATION_INDEX.md`
- Full reference: `AGENT_KERNEL_COMMUNICATION.md`
- Quick reference: `AGENT_KERNEL_COMMUNICATION_QUICK_REFERENCE.md`

---

## How to Use These Documents

### For Learning the Architecture
1. Start with QUICK_REFERENCE.md Section 1 (answers to 6 questions)
2. Review QUICK_REFERENCE.md Section 2 (message flow diagram)
3. Read AGENT_KERNEL_COMMUNICATION.md Sections 1-4 for protocol details

### For Implementation
1. Check QUICK_REFERENCE.md Section 7 (code patterns)
2. Find exact line numbers in AGENT_KERNEL_COMMUNICATION.md
3. Review Section 6 for command execution flow
4. Reference Section 9 for security requirements

### For Debugging
1. Use QUICK_REFERENCE.md Section 11 (debugging commands)
2. Check timings in QUICK_REFERENCE.md Section 5
3. Review error handling in AGENT_KERNEL_COMMUNICATION.md Section 8
4. Check resilience patterns in QUICK_REFERENCE.md Section 9

### For Extension
1. See QUICK_REFERENCE.md Section 13 (how to extend)
2. Find exact locations in AGENT_KERNEL_COMMUNICATION.md
3. Review contract system in Section 7
4. Check message format examples in Section 4

---

## Key Insights

**Architecture Principle**: Strict hierarchy
- Kernel thinks, agents obey
- Agents publish telemetry, kernel consumes
- Kernel is single source of truth

**Communication Pattern**: Event-driven, asynchronous
- No blocking calls between components
- Messages flow through MQTT broker
- Command-response correlation via command_id

**Resilience**: Multi-layered health monitoring
- Agent heartbeats detect agent failures
- MQTT keep-alive detects broker issues
- Kernel monitoring detects heartbeat gaps
- All failures gracefully degrade to offline state

**Security**: MAC-based identification
- Agent ID = MAC address (no colons)
- Commands filtered by target agent_id
- Shell commands whitelisted
- Output sanitized

**Performance**: Minimal overhead
- 30-second heartbeat (configurable)
- Small JSON payloads (2-3 KB)
- Async I/O (tokio)
- Efficient lock patterns

---

## Related Documentation

See also in the repository:
- `CLAUDE.md` - Overall Symbion system architecture
- `contracts/mqtt/` - JSON contract files for each MQTT topic
- `symbion-kernel/src/health.rs` - Health monitoring implementation
- `symbion-agent-host/src/discovery.rs` - Network discovery logic

---

## Questions or Feedback?

These documents were generated from:
- `symbion-agent-host/src/main.rs` (1189 lines)
- `symbion-kernel/src/mqtt.rs` (189 lines)
- `symbion-kernel/src/agents.rs` (624 lines)
- Related supporting modules

For the most current and authoritative information, refer to the source code directly.

---

**Generated**: November 14, 2025
**Exploration Scope**: Agent-Kernel communication architecture
**Documentation Files**: 2 comprehensive guides
