Send email via msmtp (Gmail configured).

**Usage**: `/mail [subject] [body]`

**Purpose**: Quick email notifications for important events (deployments, errors, summaries)

**Workflow**:
1. Parse subject and body from user query
2. Create email with proper headers
3. Send via msmtp (configured for Markchavatte@gmail.com)
4. Confirm delivery

**Email Configuration**:
- From: Symbion System <Markchavatte@gmail.com>
- To: Markchavatte@gmail.com (default)
- SMTP: Gmail (smtp.gmail.com:587)
- Auth: ~/.msmtprc

**Examples**:
- `/mail "Deploy Success" "Symbion v1.1.7 deployed successfully"`
- `/mail "Security Alert" "Failed login attempts detected"`
- `/mail "Summary" "Weekly project status report"`

**Auto-send scenarios** (no user prompt):
- Deployment confirmations
- Critical security events
- Scheduled reports/summaries
- System errors requiring attention

**Template**:
```
From: Symbion System <Markchavatte@gmail.com>
To: Markchavatte@gmail.com
Subject: [SUBJECT]

[BODY]

---
Sent from Symbion Automation System
Timestamp: [ISO8601]
```

Execute email send now based on user request.
