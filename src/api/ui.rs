use axum::response::Html;

const DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Orchepy Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Helvetica Neue', Arial, sans-serif;
            background: #f5f7fa;
            min-height: 100vh;
            padding: 24px;
            color: #1a202c;
        }
        .container { max-width: 100%; margin: 0 auto; }
        header {
            background: white;
            padding: 24px 32px;
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.06);
            margin-bottom: 24px;
            border: 1px solid #e2e8f0;
        }
        h1 {
            color: #2d3748;
            font-size: 24px;
            font-weight: 600;
            margin-bottom: 4px;
            letter-spacing: -0.025em;
        }
        .subtitle {
            color: #718096;
            font-size: 14px;
            font-weight: 400;
        }
        .workflows-container {
            margin-bottom: 32px;
        }
        .workflow-section {
            margin-bottom: 40px;
        }
        .workflow-header-section {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 16px 0;
            margin-bottom: 16px;
        }
        .workflow-title-group {
            display: flex;
            align-items: center;
            gap: 12px;
        }
        .workflow-title {
            font-size: 18px;
            font-weight: 600;
            color: #2d3748;
        }
        .workflow-id {
            font-size: 11px;
            color: #a0aec0;
            font-family: 'SF Mono', Monaco, Menlo, monospace;
        }
        .badge {
            padding: 3px 10px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 500;
            text-transform: uppercase;
            letter-spacing: 0.025em;
        }
        .badge-active {
            background: #c6f6d5;
            color: #22543d;
        }
        .badge-inactive {
            background: #fed7d7;
            color: #742a2a;
        }
        .kanban-board {
            display: flex;
            gap: 16px;
            overflow-x: auto;
            padding-bottom: 16px;
        }
        .kanban-column {
            background: #f7fafc;
            border-radius: 8px;
            min-width: 300px;
            flex-shrink: 0;
            max-height: 70vh;
            display: flex;
            flex-direction: column;
            border: 1px solid #e2e8f0;
        }
        .column-header {
            padding: 16px;
            border-bottom: 2px solid #e2e8f0;
            background: white;
            border-radius: 8px 8px 0 0;
        }
        .column-title {
            font-size: 14px;
            font-weight: 600;
            color: #2d3748;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin-bottom: 4px;
        }
        .column-count {
            font-size: 13px;
            color: #718096;
            font-weight: 500;
        }
        .column-cards {
            padding: 12px;
            overflow-y: auto;
            flex: 1;
        }
        .case-card {
            background: white;
            border-radius: 6px;
            padding: 12px;
            margin-bottom: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.06);
            border: 1px solid #e2e8f0;
            transition: all 0.15s ease;
            cursor: pointer;
        }
        .case-card:hover {
            box-shadow: 0 4px 12px rgba(0,0,0,0.08);
            border-color: #cbd5e0;
        }
        .case-card-id {
            font-size: 11px;
            font-family: 'SF Mono', Monaco, Menlo, monospace;
            color: #718096;
            margin-bottom: 8px;
        }
        .case-card-data {
            font-size: 12px;
            color: #4a5568;
            line-height: 1.5;
        }
        .case-card-status {
            margin-top: 8px;
            padding-top: 8px;
            border-top: 1px solid #edf2f7;
            font-size: 11px;
            color: #718096;
        }
        .case-status-badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 10px;
            font-weight: 500;
            text-transform: uppercase;
            letter-spacing: 0.025em;
        }
        .status-active {
            background: #bee3f8;
            color: #2c5282;
        }
        .status-completed {
            background: #c6f6d5;
            color: #22543d;
        }
        .status-failed {
            background: #fed7d7;
            color: #742a2a;
        }
        .status-paused {
            background: #feebc8;
            color: #7c2d12;
        }
        .loading {
            text-align: center;
            padding: 48px;
            color: #718096;
            font-size: 15px;
            font-weight: 500;
        }
        .empty-state {
            text-align: center;
            padding: 64px 24px;
            background: white;
            border-radius: 8px;
            border: 1px solid #e2e8f0;
        }
        .empty-state h2 {
            color: #4a5568;
            font-size: 18px;
            font-weight: 500;
        }
        .empty-column {
            text-align: center;
            padding: 24px 12px;
            color: #a0aec0;
            font-size: 13px;
        }
        .refresh-btn {
            position: fixed;
            bottom: 24px;
            right: 24px;
            background: white;
            border: 1px solid #e2e8f0;
            padding: 12px 20px;
            border-radius: 8px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.1);
            cursor: pointer;
            font-weight: 500;
            font-size: 14px;
            color: #2d3748;
            transition: all 0.15s ease;
        }
        .refresh-btn:hover {
            box-shadow: 0 6px 16px rgba(0,0,0,0.12);
            border-color: #cbd5e0;
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>Orchepy Dashboard</h1>
            <p class="subtitle">Real-time Kanban view of workflows and cases</p>
        </header>
        <div id="loading" class="loading">Loading workflows...</div>
        <div id="workflows" class="workflows-container"></div>
    </div>
    <button class="refresh-btn" onclick="loadWorkflows()">Refresh</button>

    <script>
        async function loadWorkflows() {
            const loading = document.getElementById('loading');
            const workflowsContainer = document.getElementById('workflows');
            loading.style.display = 'block';
            workflowsContainer.innerHTML = '';

            try {
                const response = await fetch('/workflows');
                const workflows = await response.json();
                loading.style.display = 'none';

                if (workflows.length === 0) {
                    workflowsContainer.innerHTML = '<div class="empty-state"><h2>No workflows found</h2></div>';
                    return;
                }

                for (const workflow of workflows) {
                    await renderWorkflowKanban(workflow, workflowsContainer);
                }
            } catch (err) {
                loading.innerHTML = 'Failed to load: ' + err.message;
            }
        }

        async function renderWorkflowKanban(workflow, container) {
            const section = document.createElement('div');
            section.className = 'workflow-section';

            const statusBadge = workflow.active
                ? '<span class="badge badge-active">Active</span>'
                : '<span class="badge badge-inactive">Inactive</span>';

            section.innerHTML = `
                <div class="workflow-header-section">
                    <div class="workflow-title-group">
                        <div>
                            <div class="workflow-title">${workflow.name}</div>
                            <div class="workflow-id">${workflow.id}</div>
                        </div>
                        ${statusBadge}
                    </div>
                </div>
                <div class="kanban-board" id="kanban-${workflow.id}"></div>
            `;

            container.appendChild(section);

            try {
                const casesResponse = await fetch(`/cases?workflow_id=${workflow.id}`);
                const cases = await casesResponse.json();

                const kanbanBoard = document.getElementById(`kanban-${workflow.id}`);
                const phases = workflow.phases || [];

                phases.forEach(phase => {
                    const phaseCases = cases.filter(c => c.current_phase === phase);
                    const column = document.createElement('div');
                    column.className = 'kanban-column';

                    column.innerHTML = `
                        <div class="column-header">
                            <div class="column-title">${phase}</div>
                            <div class="column-count">${phaseCases.length} ${phaseCases.length === 1 ? 'case' : 'cases'}</div>
                        </div>
                        <div class="column-cards" id="column-${workflow.id}-${phase}"></div>
                    `;

                    kanbanBoard.appendChild(column);

                    const cardsContainer = document.getElementById(`column-${workflow.id}-${phase}`);
                    if (phaseCases.length === 0) {
                        cardsContainer.innerHTML = '<div class="empty-column">No cases in this phase</div>';
                    } else {
                        phaseCases.forEach(caseItem => {
                            const card = createCaseCard(caseItem);
                            cardsContainer.appendChild(card);
                        });
                    }
                });
            } catch (err) {
                console.error('Failed to load cases for workflow:', workflow.id, err);
            }
        }

        function createCaseCard(caseItem) {
            const card = document.createElement('div');
            card.className = 'case-card';

            const statusClass = `status-${caseItem.status}`;
            const dataPreview = formatDataPreview(caseItem.data);
            const timeAgo = formatTimeAgo(new Date(caseItem.created_at));

            card.innerHTML = `
                <div class="case-card-id">${caseItem.id.split('-')[0]}</div>
                <div class="case-card-data">${dataPreview}</div>
                <div class="case-card-status">
                    <span class="case-status-badge ${statusClass}">${caseItem.status}</span>
                    <span style="margin-left: 8px;">${timeAgo}</span>
                </div>
            `;

            return card;
        }

        function formatDataPreview(data) {
            if (!data || typeof data !== 'object') return 'No data';

            const entries = Object.entries(data).slice(0, 2);
            if (entries.length === 0) return 'Empty data';

            return entries.map(([key, value]) => {
                const displayValue = typeof value === 'object' ? JSON.stringify(value) : String(value);
                const truncated = displayValue.length > 30 ? displayValue.substring(0, 30) + '...' : displayValue;
                return `<strong>${key}:</strong> ${truncated}`;
            }).join('<br>');
        }

        function formatTimeAgo(date) {
            const seconds = Math.floor((new Date() - date) / 1000);

            if (seconds < 60) return 'just now';
            const minutes = Math.floor(seconds / 60);
            if (minutes < 60) return `${minutes}m ago`;
            const hours = Math.floor(minutes / 60);
            if (hours < 24) return `${hours}h ago`;
            const days = Math.floor(hours / 24);
            if (days < 7) return `${days}d ago`;
            return date.toLocaleDateString();
        }

        setInterval(loadWorkflows, 30000);
        loadWorkflows();
    </script>
</body>
</html>
"#;

pub async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}
