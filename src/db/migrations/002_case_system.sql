DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'case_status') THEN
        CREATE TYPE case_status AS ENUM ('active', 'completed', 'failed', 'paused');
    END IF;
END$$;
CREATE TABLE IF NOT EXISTS orchepy_workflows (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    phases JSONB NOT NULL,
    initial_phase VARCHAR(255) NOT NULL,
    webhook_url TEXT,
    automations JSONB,
    sla_config JSONB,
    active BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS orchepy_cases (
    id UUID PRIMARY KEY,
    workflow_id UUID NOT NULL REFERENCES orchepy_workflows(id) ON DELETE CASCADE,
    current_phase VARCHAR(255) NOT NULL,
    previous_phase VARCHAR(255),
    data JSONB NOT NULL DEFAULT '{}',
    status case_status NOT NULL DEFAULT 'active',
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    phase_entered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS orchepy_case_history (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES orchepy_cases(id) ON DELETE CASCADE,
    from_phase VARCHAR(255),
    to_phase VARCHAR(255) NOT NULL,
    reason TEXT,
    triggered_by VARCHAR(255),
    transitioned_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_orchepy_cases_workflow ON orchepy_cases (workflow_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_orchepy_cases_status ON orchepy_cases (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_orchepy_cases_current_phase ON orchepy_cases (current_phase, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_orchepy_cases_workflow_phase ON orchepy_cases (workflow_id, current_phase);
CREATE INDEX IF NOT EXISTS idx_orchepy_cases_created_at ON orchepy_cases (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_orchepy_cases_phase_entered ON orchepy_cases (phase_entered_at);
CREATE INDEX IF NOT EXISTS idx_orchepy_case_history_case_id ON orchepy_case_history (case_id, transitioned_at DESC);
CREATE INDEX IF NOT EXISTS idx_orchepy_case_history_phases ON orchepy_case_history (from_phase, to_phase);
CREATE INDEX IF NOT EXISTS idx_orchepy_workflows_active ON orchepy_workflows (active);
CREATE INDEX IF NOT EXISTS idx_orchepy_workflows_created ON orchepy_workflows (created_at DESC);
CREATE OR REPLACE FUNCTION update_case_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER trigger_update_case_updated_at
    BEFORE UPDATE ON orchepy_cases
    FOR EACH ROW
    EXECUTE FUNCTION update_case_updated_at();
CREATE OR REPLACE TRIGGER update_workflows_updated_at
    BEFORE UPDATE ON orchepy_workflows
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
CREATE OR REPLACE FUNCTION track_case_phase_change()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.current_phase IS DISTINCT FROM NEW.current_phase THEN
        NEW.phase_entered_at = NOW();
        INSERT INTO orchepy_case_history (
            id,
            case_id,
            from_phase,
            to_phase,
            transitioned_at
        ) VALUES (
            gen_random_uuid(),
            NEW.id,
            OLD.current_phase,
            NEW.current_phase,
            NOW()
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER trigger_track_case_phase_change
    BEFORE UPDATE ON orchepy_cases
    FOR EACH ROW
    EXECUTE FUNCTION track_case_phase_change();
