# Orchepy

A lightweight, domain-agnostic event orchestrator for workflow automation.

## Usage

### 1. Create a Workflow

```bash
curl -X POST http://localhost:3296/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Sales Pipeline",
    "phases": ["Lead", "Qualified", "Proposal", "Negotiation", "Closed"],
    "initial_phase": "Lead",
    "active": true,
    "webhook_url": "https://your-webhook.com/notifications"
  }'
```

### 1.1. Create a Workflow with Automations

Automations allow you to execute actions automatically when cases enter or exit specific phases:

```bash
curl -X POST http://localhost:3296/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Invoice Processing",
    "phases": ["Upload", "OCR", "Validation", "Manual Review", "Approved"],
    "initial_phase": "Upload",
    "active": true,
    "automations": {
      "automations": [
        {
          "trigger": "on_enter",
          "phase": "OCR",
          "actions": [
            {
              "type": "webhook",
              "id": "ocr_process",
              "name": "Start OCR Processing",
              "url": "https://ocr-service.com/process",
              "method": "POST",
              "headers": {
                "Authorization": "Bearer YOUR_TOKEN",
                "Content-Type": "application/json"
              },
              "fields": ["case_id", "data"],
              "retry": {
                "enabled": true,
                "max_attempts": 3,
                "delay_ms": 1000
              },
              "on_error": "stop"
            },
            {
              "type": "delay",
              "name": "Wait for OCR",
              "duration_ms": 5000
            }
          ]
        },
        {
          "trigger": "on_enter",
          "phase": "Validation",
          "actions": [
            {
              "type": "webhook",
              "name": "Validate document",
              "url": "https://validation-service.com/validate",
              "method": "POST",
              "use_response_from": "ocr_process",
              "on_error": "continue"
            }
          ]
        },
        {
          "trigger": "on_exit",
          "phase": "Manual Review",
          "actions": [
            {
              "type": "webhook",
              "name": "Log review completion",
              "url": "https://logging-service.com/log",
              "fields": ["case_id", "current_phase", "status"]
            }
          ]
        }
      ]
    }
  }'
```

Automation Features:

- Triggers: `on_enter` (when case enters phase) or `on_exit` (when case exits phase)
- Action Types:
    - `webhook`: HTTP call to external API
    - `delay`: Wait for specified milliseconds
    - `conditional`: Execute actions based on conditions (supports AND/OR logic)
    - `move_to_phase`: Automatically move case to another phase
    - `set_field`: Update case data fields
- Webhook Options:
    - `fields`: Send only specific case fields (if omitted, sends entire case)
    - `headers`: Custom HTTP headers (e.g., Authorization)
    - `retry`: Automatic retry with configurable attempts and delay
    - `on_error`: "stop" (halt execution) or "continue" (log and proceed)
    - `use_response_from`: Chain actions by using response from previous action
- Execution: Actions run sequentially in the order defined

### 1.2. Conditional Actions

Execute different actions based on case data:

```bash
curl -X POST http://localhost:3296/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Approval Workflow",
    "phases": ["Pending", "Review", "Approved", "Rejected"],
    "initial_phase": "Pending",
    "automations": {
      "automations": [
        {
          "trigger": "on_enter",
          "phase": "Review",
          "actions": [
            {
              "type": "conditional",
              "operator": "AND",
              "conditions": [
                {"field": "data.amount", "op": ">", "value": 10000},
                {"field": "status", "op": "==", "value": "active"}
              ],
              "then": [
                {
                  "type": "webhook",
                  "url": "https://manager-approval.com/request"
                },
                {
                  "type": "move_to_phase",
                  "phase": "Approved"
                }
              ],
              "else": [
                {
                  "type": "set_field",
                  "field": "data.auto_approved",
                  "value": true
                },
                {
                  "type": "move_to_phase",
                  "phase": "Approved"
                }
              ]
            }
          ]
        }
      ]
    }
  }'
```

Supported Operators: `==`, `!=`, `>`, `<`, `>=`, `<=`, `contains`

Logical Operators: `AND`, `OR` (for complex conditions)

Simple Condition:

```json
{
  "type": "conditional",
  "field": "data.amount",
  "operator": ">",
  "value": 5000,
  "then": [...],
  "else": [...]
}
```

### 1.3. SLA Configuration

Set time limits for each phase to track compliance:

```bash
curl -X POST http://localhost:3296/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Support Tickets",
    "phases": ["New", "In Progress", "Resolved"],
    "initial_phase": "New",
    "sla_config": {
      "New": {"hours": 2},
      "In Progress": {"hours": 24},
      "Resolved": {"hours": 48}
    }
  }'
```

Cases track when they entered each phase via `phase_entered_at` timestamp.

### 2. Create a Case

```bash
curl -X POST http://localhost:3296/cases \
  -H "Content-Type: application/json" \
  -d '{
    "workflow_id": "WORKFLOW_ID_HERE",
    "data": {
      "customer": "Acme Corp",
      "value": 50000,
      "contact": "john@acme.com"
    },
    "metadata": {
      "source": "website",
      "campaign": "Q4-2024"
    }
  }'
```

### 3. Move Case Between Phases

```bash
curl -X PUT http://localhost:3296/cases/CASE_ID/move \
  -H "Content-Type: application/json" \
  -d '{
    "to_phase": "Qualified",
    "reason": "Customer showed interest in enterprise plan",
    "triggered_by": "sales-agent-123"
  }'
```

### 4. Update Case Data

```bash
curl -X PATCH http://localhost:3296/cases/CASE_ID/data \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "value": 75000,
      "notes": "Upgraded to premium package"
    }
  }'
```

### 5. List Cases by Phase

```bash
# Get all cases in a specific phase
curl "http://localhost:3296/cases?workflow_id=WORKFLOW_ID&current_phase=Negotiation"

# Get all cases with filters
curl "http://localhost:3296/cases?workflow_id=WORKFLOW_ID&status=active&limit=50"
```

### 6. View Case History

```bash
curl http://localhost:3296/cases/CASE_ID/history
```

### 7. Access Kanban Dashboard

Open your browser and navigate to:

```
http://localhost:3296/
```

This displays a real-time Kanban board showing all workflows and their cases organized by phases.

### Event-Driven Workflows

Orchepy automatically triggers webhooks when cases are created or moved between phases:

Case Created Event:

```json
{
  "event_type": "case.created",
  "data": {
    "case_id": "uuid",
    "workflow_id": "uuid",
    "to_phase": "Lead",
    "from_phase": null,
    "case_data": { ... }
  }
}
```

Case Moved Event:

```json
{
  "event_type": "case.moved",
  "data": {
    "case_id": "uuid",
    "workflow_id": "uuid",
    "to_phase": "Qualified",
    "from_phase": "Lead",
    "case_data": { ... }
  }
}
```

## Configuration

### Environment Variables

```bash
DATABASE_URL=postgres://user:password@localhost:5432/dbname
HOST=0.0.0.0
PORT=3296
RUST_LOG=info,orchepy=debug

WHITELIST_ENABLED=false
WHITELIST_IPS=192.168.1.100,10.0.0.50

WEBHOOK_ON_CASE_CREATE=true
WEBHOOK_ON_CASE_MOVE=true
```

Webhook Control:

- `WEBHOOK_ON_CASE_CREATE`: Enable/disable global webhooks when cases are created
- `WEBHOOK_ON_CASE_MOVE`: Enable/disable global webhooks when cases move between phases

These settings control the workflow's `webhook_url` field. Automations are independent and always execute when configured.

## Database Tables

- `orchepy_workflows`: Workflow definitions
- `orchepy_cases`: Case instances
- `orchepy_case_history`: Phase transition history
- `orchepy_events`: External events (for workflow engine)
- `orchepy_flows`: Flow definitions (for workflow engine)
- `orchepy_executions`: Flow execution logs

## License

[MIT](./LICENSE)
