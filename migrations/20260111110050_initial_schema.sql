-- Add migration script here
CREATE TABLE IF NOT EXISTS vaults(
  vault_pubkey TEXT PRIMARY KEY,
  owner_pubkey TEXT NOT NULL,
  token_account TEXT NOT NULL,
  total_balance BIGINT NOT NULL DEFAULT 0,
  locked_balance BIGINT NOT NULL DEFAULT 0,
  available_balance BIGINT GENERATED ALWAYS AS (total_balance - locked_balance) STORED,
  total_deposited BIGINT NOT NULL DEFAULT 0,
  total_withdrawn BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL,
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

  CONSTRAINT positive_balances CHECK (total_balance >= 0 AND locked_balance >= 0 AND available_balance >= 0)
);

CREATE INDEX IF NOT EXISTS idx_vaults_owner ON vaults(owner_pubkey);
CREATE INDEX IF NOT EXISTS idx_vaults_updates ON vaults(updated_at); -- idx -> index vaults -> table_name updates -> random name 

CREATE TABLE IF NOT EXISTS transactions(
  id BIGSERIAL PRIMARY KEY,
  vault_pubkey TEXT NOT NULL REFERENCES vaults(vault_pubkey),
  tx_signature TEXT UNIQUE NOT NULL,
  tx_type TEXT NOT NULL,
  amount BIGINT NOT NULL,
  from_vault TEXT,
  to_vault TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  block_time BIGINT,
  slot BIGINT,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  confirmed_at TIMESTAMP WITH TIME ZONE,
  meta JSONB,

  CONSTRAINT valid_tx_type CHECK (tx_type IN ('deposit' , 'withdraw' , 'lock' , 'unlock' , 'transfer')),
  CONSTRAINT valid_status CHECK (status IN ('pending' , 'confirmed' , 'failed')),
  CONSTRAINT positive_amount CHECK (amount > 0)
);

CREATE INDEX IF NOT EXISTS idx_transactions_vault ON transactions(vault_pubkey);
CREATE INDEX IF NOT EXISTS idx_transactions_signature ON transactions(tx_signature);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(tx_type);
CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status); 
CREATE INDEX IF NOT EXISTS idx_transactions_created ON transactions(created_at); 

CREATE TABLE IF NOT EXISTS balance_snapshots(
  id BIGSERIAL PRIMARY KEY,
  vault_pubkey TEXT NOT NULL REFERENCES vaults(vault_pubkey),
  total_balance BIGINT NOT NULL,
  locked_balance BIGINT NOT NULL,
  available_balance BIGINT NOT NULL,
  on_chain_token_balance BIGINT NOT NULL,
  snapshot_type TEXT NOT NULL,
  snapshot_ts TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  discrepancy BIGINT GENERATED ALWAYS AS (total_balance - on_chain_token_balance) STORED,

  CONSTRAINT valid_snapshot_type CHECK (snapshot_type IN ('hourly' , 'daily' , 'reconciliation'))
);

CREATE INDEX IF NOT EXISTS idx_snapshots_vault ON balance_snapshots(vault_pubkey); 
CREATE INDEX IF NOT EXISTS idx_snapshots_ts ON balance_snapshots(snapshot_ts);
CREATE INDEX IF NOT EXISTS idx_snapshots_type ON balance_snapshots(snapshot_type);
CREATE INDEX IF NOT EXISTS idx_snapshots_discrepancy ON balance_snapshots(discrepancy);

CREATE TABLE IF NOT EXISTS reconciliation_logs(
  id BIGSERIAL PRIMARY KEY,
  vault_pubkey TEXT NOT NULL REFERENCES vaults(vault_pubkey),
  expected_balance BIGINT NOT NULL,
  actual_balance BIGINT NOT NULL,
  discrepancy BIGINT NOT NULL,
  resolution_status TEXT NOT NULL DEFAULT 'detected',
  resolution_notes TEXT,
  detected_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  resolved_at TIMESTAMP WITH TIME ZONE,
  
  CONSTRAINT valid_resolution_status CHECK (resolution_status IN ('detected' , 'investigating' , 'resolved'))
);

CREATE INDEX IF NOT EXISTS idx_reconciliation_vault ON reconciliation_logs(vault_pubkey);
CREATE INDEX IF NOT EXISTS idx_reconciliation_status ON reconciliation_logs(resolution_status);
CREATE INDEX IF NOT EXISTS idx_reconciliation_detected ON reconciliation_logs(detected_at);

CREATE TABLE IF NOT EXISTS audit_trail (
  id BIGSERIAL PRIMARY KEY,
  event_type TEXT NOT NULL,
  vault_pubkey TEXT,
  user_pubkey TEXT,
  amount BIGINT,
  tx_signature TEXT,
  event_data JSONB NOT NULL,
  ip_address TEXT,
  user_agent TEXT,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

  CONSTRAINT valid_event_type CHECK (event_type IN (
    'balance_change','vault_created' , 'deposit' , 'withdraw' , 'lock' , 'unlock' , 'transfer' , 'reconciliation' , 'alert' , 'error'
  ))
);

CREATE INDEX IF NOT EXISTS idx_audit_type ON audit_trail(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_vault ON audit_trail(vault_pubkey);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_trail(user_pubkey);
CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_trail(created_at);

CREATE TABLE IF NOT EXISTS alerts(
  id BIGSERIAL PRIMARY KEY,
  alert_type TEXT NOT NULL,
  severity TEXT NOT NULL,
  vault_pubkey TEXT,
  message TEXT NOT NULL,
  details JSONB,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  acknowledged_at TIMESTAMP WITH TIME ZONE,
  resolved_at TIMESTAMP WITH TIME ZONE,

  CONSTRAINT valid_severity CHECK (severity IN ('INFO' , 'warning' , 'critical')),
  CONSTRAINT valid_alert_status CHECK (status IN ('active' , 'acknowledged' , 'resolved'))
);

CREATE INDEX IF NOT EXISTS idx_alerts_type ON alerts(alert_type);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status);
CREATE INDEX IF NOT EXISTS idx_alerts_vault ON alerts(vault_pubkey);
CREATE INDEX IF NOT EXISTS idx_alerts_created ON alerts(created_at);

CREATE OR REPLACE VIEW tvl_stats AS
SELECT
  COUNT(*) AS total_vaults,
  SUM(total_balance) AS total_value_locked, 
  SUM(locked_balance) AS total_available,
  SUM(available_balance) AS total_locked,
  AVG(total_balance) AS avg_vault_balance,
  MAX(total_balance) AS max_vault_balance,
  NOW() AS calculated_at
FROM vaults;

CREATE OR REPLACE FUNCTION update_vault_timestamp()
RETURNS TRIGGER AS $$
BEGIN 
  NEW.updated_at := NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS vault_update_timestamp ON vaults;
CREATE TRIGGER vault_update_timestamp
  BEFORE UPDATE ON vaults
  FOR EACH ROW
  EXECUTE FUNCTION update_vault_timestamp();

CREATE OR REPLACE FUNCTION create_vault_audits()
RETURNS TRIGGER AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    INSERT INTO audit_trail (event_type, vault_pubkey, user_pubkey, event_data)
    VALUES ('vault_created' , NEW.vault_pubkey , NEW.owner_pubkey , jsonb_build_object('total_balance' ,NEW.total_balance));
  ELSEIF TG_OP = 'UPDATE' THEN
    IF NEW.total_balance != OLD.total_balance THEN
      INSERT INTO audit_trail (event_type, vault_pubkey, amount, event_data)
      VALUES ('balance_change', NEW.vault_pubkey, 
              NEW.total_balance - OLD.total_balance,
              jsonb_build_object(
                  'old_balance', OLD.total_balance,
                  'new_balance', NEW.total_balance,
                  'old_locked', OLD.locked_balance,
                  'new_locked', NEW.locked_balance
              ));
      END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS vault_audit_trigger ON vaults;
CREATE TRIGGER vault_audit_trigger
    AFTER INSERT OR UPDATE ON vaults
    FOR EACH ROW
    EXECUTE FUNCTION create_vault_audits();
