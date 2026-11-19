-- 0005_sla.sql — SLA
-- sla_config, sla_tracking

CREATE TABLE sla_config (
    id    BIGSERIAL PRIMARY KEY,
    step  TEXT NOT NULL,
    hari  SMALLINT NOT NULL,
    basis TEXT NOT NULL DEFAULT 'KERJA' CHECK (basis IN ('KERJA','KALENDER')),
    aktif BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE sla_tracking (
    id           BIGSERIAL PRIMARY KEY,
    ref_type     TEXT NOT NULL,
    ref_id       BIGINT NOT NULL,
    step         TEXT NOT NULL,
    deadline     TIMESTAMPTZ NOT NULL,
    status       TEXT NOT NULL DEFAULT 'ON_TIME'
                     CHECK (status IN ('ON_TIME','OVERDUE','ESCALATED')),
    escalated_to BIGINT,
    escalated_at TIMESTAMPTZ
);
