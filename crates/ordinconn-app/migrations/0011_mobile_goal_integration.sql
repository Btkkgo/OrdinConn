-- Each real request attempt spends the canonical Goal budget, even on failure.
CREATE TABLE mobile_model_calls (
 id TEXT PRIMARY KEY,
 goal_id TEXT NOT NULL REFERENCES mobile_goals(id),
 observation_id TEXT REFERENCES mobile_observations(id),
 created_at TEXT NOT NULL
);
CREATE INDEX mobile_model_calls_goal ON mobile_model_calls(goal_id);
CREATE TRIGGER mobile_model_calls_no_update BEFORE UPDATE ON mobile_model_calls BEGIN SELECT RAISE(ABORT,'model budget immutable'); END;
CREATE TRIGGER mobile_model_calls_no_delete BEFORE DELETE ON mobile_model_calls BEGIN SELECT RAISE(ABORT,'model budget retained'); END;
CREATE TABLE mobile_goal_completion_targets (
 goal_id TEXT PRIMARY KEY REFERENCES mobile_goals(id),
 target_json TEXT NOT NULL CHECK(json_valid(target_json))
);
CREATE TRIGGER mobile_goal_target_no_update BEFORE UPDATE ON mobile_goal_completion_targets BEGIN SELECT RAISE(ABORT,'owner target immutable'); END;
CREATE TRIGGER mobile_goal_target_no_delete BEFORE DELETE ON mobile_goal_completion_targets BEGIN SELECT RAISE(ABORT,'owner target retained'); END;
CREATE TABLE mobile_step_approvals (
 id TEXT PRIMARY KEY,
 goal_id TEXT NOT NULL REFERENCES mobile_goals(id),
 plan_id TEXT NOT NULL REFERENCES mobile_plans(id),
 step_id TEXT NOT NULL UNIQUE REFERENCES mobile_plan_steps(id),
 status TEXT NOT NULL CHECK(status IN ('requested','authorized','claimed')),
 request_json TEXT NOT NULL CHECK(json_valid(request_json))
);
DROP TRIGGER mobile_goal_transition;
CREATE TRIGGER mobile_goal_transition BEFORE UPDATE OF status ON mobile_goals
WHEN OLD.status <> NEW.status AND NOT (
 (OLD.status='PENDING' AND NEW.status IN ('PLANNING','FAILED','STOPPED')) OR
 (OLD.status='PLANNING' AND NEW.status IN ('RUNNING','WAITING_APPROVAL','FAILED','STOPPED')) OR
 (OLD.status='RUNNING' AND NEW.status IN ('WAITING_APPROVAL','COMPLETED','FAILED','STOPPED')) OR
 (OLD.status='WAITING_APPROVAL' AND NEW.status IN ('RUNNING','FAILED','STOPPED'))
) BEGIN SELECT RAISE(ABORT,'invalid goal state transition'); END;
DROP TRIGGER mobile_step_transition;
CREATE TRIGGER mobile_step_transition BEFORE UPDATE OF status ON mobile_plan_steps
WHEN OLD.status <> NEW.status AND NOT (
 (OLD.status='PENDING' AND NEW.status IN ('EXECUTING','WAITING_APPROVAL','SKIPPED','STOPPED')) OR
 (OLD.status='WAITING_APPROVAL' AND NEW.status IN ('EXECUTING','FAILED','STOPPED')) OR
 (OLD.status='WAITING_APPROVAL' AND NEW.status='PENDING' AND EXISTS (
   SELECT 1 FROM mobile_step_approvals a WHERE a.step_id=OLD.id AND a.status='authorized'
   AND json_extract(a.request_json,'$.tokenState')='consumed')) OR
 (OLD.status='EXECUTING' AND NEW.status IN ('VERIFIED','FAILED','STOPPED'))
) BEGIN SELECT RAISE(ABORT,'invalid step state transition'); END;
