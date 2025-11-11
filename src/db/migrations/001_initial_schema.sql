DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'execution_status') THEN
        CREATE TYPE execution_status AS ENUM ('pending', 'running', 'completed', 'failed', 'retrying');
    END IF;
END$$;
CREATE TABLE IF NOT EXISTS orchepy_events (
    id UUID PRIMARY KEY,
    event_type VARCHAR(255) NOT NULL,
    data JSONB NOT NULL,
    metadata JSONB,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orchepy_events_type_received ON orchepy_events (event_type, received_at DESC);

CREATE TABLE IF NOT EXISTS orchepy_flows (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    trigger JSONB NOT NULL,
    steps JSONB NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orchepy_flows_active ON orchepy_flows (active);
CREATE INDEX idx_orchepy_flows_created ON orchepy_flows (created_at DESC);

CREATE TABLE IF NOT EXISTS orchepy_executions (
    id UUID PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES orchepy_flows(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES orchepy_events(id) ON DELETE CASCADE,
    status execution_status NOT NULL DEFAULT 'pending',
    current_step VARCHAR(255),
    steps_status JSONB NOT NULL DEFAULT '{}',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error TEXT
);

CREATE INDEX idx_orchepy_executions_flow ON orchepy_executions (flow_id, started_at DESC);
CREATE INDEX idx_orchepy_executions_status ON orchepy_executions (status, started_at DESC);
CREATE INDEX idx_orchepy_executions_event ON orchepy_executions (event_id);

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_orchepy_flows_updated_at
    BEFORE UPDATE ON orchepy_flows
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
