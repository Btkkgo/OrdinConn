-- Execution contracts only; no executor or implicit conversion of research tasks.
CREATE TABLE mobile_goals (
  id TEXT PRIMARY KEY,
  status TEXT NOT NULL CHECK(status IN ('PENDING','PLANNING','RUNNING','WAITING_APPROVAL','COMPLETED','FAILED','STOPPED')),
  active_plan_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  domain_json TEXT NOT NULL CHECK(json_valid(domain_json)),
  CHECK(json_extract(domain_json,'$.id') IS id),
  CHECK(json_extract(domain_json,'$.status') IS status),
  CHECK(json_extract(domain_json,'$.activePlanId') IS active_plan_id),
  FOREIGN KEY(id, active_plan_id) REFERENCES mobile_plans(goal_id,id) ON DELETE RESTRICT
);
CREATE TABLE mobile_plans (
  id TEXT PRIMARY KEY,
  goal_id TEXT NOT NULL REFERENCES mobile_goals(id) ON DELETE RESTRICT,
  revision INTEGER NOT NULL CHECK(revision > 0),
  status TEXT NOT NULL CHECK(status IN ('DRAFT','ACTIVE','COMPLETED','SUPERSEDED','FAILED')),
  created_at TEXT NOT NULL,
  domain_json TEXT NOT NULL CHECK(json_valid(domain_json)),
  UNIQUE(goal_id,revision), UNIQUE(goal_id,id),
  CHECK(json_extract(domain_json,'$.id') IS id),
  CHECK(json_extract(domain_json,'$.goalId') IS goal_id),
  CHECK(json_extract(domain_json,'$.revision') IS revision),
  CHECK(json_extract(domain_json,'$.status') IS status)
);
CREATE UNIQUE INDEX mobile_one_active_plan ON mobile_plans(goal_id) WHERE status='ACTIVE';
CREATE TABLE mobile_plan_steps (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES mobile_plans(id) ON DELETE RESTRICT,
  sequence INTEGER NOT NULL CHECK(sequence > 0),
  status TEXT NOT NULL CHECK(status IN ('PENDING','EXECUTING','WAITING_APPROVAL','VERIFIED','FAILED','SKIPPED','STOPPED')),
  risk TEXT NOT NULL CHECK(risk IN ('READ_ONLY','REVERSIBLE','APPROVAL_REQUIRED','FORBIDDEN')),
  observation_before_id TEXT REFERENCES mobile_observations(id) ON DELETE RESTRICT,
  observation_after_id TEXT REFERENCES mobile_observations(id) ON DELETE RESTRICT,
  action_id TEXT REFERENCES mobile_action_receipts(id) ON DELETE RESTRICT,
  evidence_id TEXT REFERENCES evidence(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL,
  domain_json TEXT NOT NULL CHECK(json_valid(domain_json)),
  UNIQUE(plan_id,sequence),
  CHECK(json_extract(domain_json,'$.id') IS id),
  CHECK(json_extract(domain_json,'$.planId') IS plan_id),
  CHECK(json_extract(domain_json,'$.sequence') IS sequence),
  CHECK(json_extract(domain_json,'$.status') IS status),
  CHECK(json_extract(domain_json,'$.risk') IS risk),
  CHECK(json_extract(domain_json,'$.observationBeforeId') IS observation_before_id),
  CHECK(json_extract(domain_json,'$.observationAfterId') IS observation_after_id),
  CHECK(json_extract(domain_json,'$.actionId') IS action_id),
  CHECK(json_extract(domain_json,'$.evidenceId') IS evidence_id)
);
CREATE UNIQUE INDEX mobile_one_executing_step ON mobile_plan_steps(plan_id) WHERE status IN ('EXECUTING','WAITING_APPROVAL');
CREATE UNIQUE INDEX mobile_step_action_owner ON mobile_plan_steps(action_id) WHERE action_id IS NOT NULL;
CREATE TABLE mobile_step_evidence (
  step_id TEXT NOT NULL REFERENCES mobile_plan_steps(id) ON DELETE RESTRICT,
  evidence_id TEXT NOT NULL REFERENCES evidence(id) ON DELETE RESTRICT,
  observation_id TEXT NOT NULL REFERENCES mobile_observations(id) ON DELETE RESTRICT,
  PRIMARY KEY(step_id,evidence_id)
);
CREATE TABLE mobile_step_results (
  step_id TEXT PRIMARY KEY REFERENCES mobile_plan_steps(id) ON DELETE RESTRICT,
  observation_before_id TEXT REFERENCES mobile_observations(id) ON DELETE RESTRICT,
  observation_after_id TEXT REFERENCES mobile_observations(id) ON DELETE RESTRICT,
  action_receipt_id TEXT REFERENCES mobile_action_receipts(id) ON DELETE RESTRICT,
  created_at TEXT NOT NULL,
  domain_json TEXT NOT NULL CHECK(json_valid(domain_json)),
  CHECK(json_extract(domain_json,'$.stepId') IS step_id),
  CHECK(json_extract(domain_json,'$.observationBeforeId') IS observation_before_id),
  CHECK(json_extract(domain_json,'$.observationAfterId') IS observation_after_id),
  CHECK(json_extract(domain_json,'$.actionReceiptId') IS action_receipt_id)
);
CREATE INDEX mobile_goals_recent ON mobile_goals(created_at DESC,id);
CREATE INDEX mobile_steps_pending ON mobile_plan_steps(plan_id,status,sequence);
CREATE TRIGGER mobile_goals_no_delete BEFORE DELETE ON mobile_goals BEGIN SELECT RAISE(ABORT,'mobile history retained'); END;
CREATE TRIGGER mobile_plans_no_delete BEFORE DELETE ON mobile_plans BEGIN SELECT RAISE(ABORT,'mobile history retained'); END;
CREATE TRIGGER mobile_steps_no_delete BEFORE DELETE ON mobile_plan_steps BEGIN SELECT RAISE(ABORT,'mobile history retained'); END;
CREATE TRIGGER mobile_results_no_delete BEFORE DELETE ON mobile_step_results BEGIN SELECT RAISE(ABORT,'mobile history retained'); END;
CREATE TRIGGER mobile_results_no_update BEFORE UPDATE ON mobile_step_results BEGIN SELECT RAISE(ABORT,'mobile results immutable'); END;
CREATE TRIGGER mobile_evidence_no_delete BEFORE DELETE ON mobile_step_evidence BEGIN SELECT RAISE(ABORT,'mobile history retained'); END;
CREATE TRIGGER mobile_evidence_no_update BEFORE UPDATE ON mobile_step_evidence BEGIN SELECT RAISE(ABORT,'mobile evidence linkage immutable'); END;
CREATE TRIGGER mobile_goal_transition BEFORE UPDATE OF status ON mobile_goals
WHEN OLD.status <> NEW.status AND NOT (
 (OLD.status='PENDING' AND NEW.status IN ('PLANNING','FAILED','STOPPED')) OR
 (OLD.status='PLANNING' AND NEW.status IN ('RUNNING','FAILED','STOPPED')) OR
 (OLD.status='RUNNING' AND NEW.status IN ('WAITING_APPROVAL','COMPLETED','FAILED','STOPPED')) OR
 (OLD.status='WAITING_APPROVAL' AND NEW.status IN ('RUNNING','FAILED','STOPPED'))
) BEGIN SELECT RAISE(ABORT,'invalid goal state transition'); END;
CREATE TRIGGER mobile_step_transition BEFORE UPDATE OF status ON mobile_plan_steps
WHEN OLD.status <> NEW.status AND NOT (
 (OLD.status='PENDING' AND NEW.status IN ('EXECUTING','WAITING_APPROVAL','SKIPPED','STOPPED')) OR
 (OLD.status='WAITING_APPROVAL' AND NEW.status IN ('EXECUTING','FAILED','STOPPED')) OR
 (OLD.status='EXECUTING' AND NEW.status IN ('VERIFIED','FAILED','STOPPED'))
) BEGIN SELECT RAISE(ABORT,'invalid step state transition'); END;
CREATE TRIGGER mobile_plan_transition BEFORE UPDATE OF status ON mobile_plans
WHEN OLD.status <> NEW.status AND NOT (
 (OLD.status='DRAFT' AND NEW.status IN ('ACTIVE','SUPERSEDED','FAILED')) OR
 (OLD.status='ACTIVE' AND NEW.status IN ('COMPLETED','SUPERSEDED','FAILED'))
) BEGIN SELECT RAISE(ABORT,'invalid plan state transition'); END;
