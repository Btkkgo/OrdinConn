-- Durable single-use execution identity. Device ownership itself is scoped by Rust RAII.
CREATE TABLE mobile_executor_attempts (
 id TEXT PRIMARY KEY,
 goal_id TEXT NOT NULL REFERENCES mobile_goals(id),
 plan_id TEXT NOT NULL REFERENCES mobile_plans(id),
 step_id TEXT NOT NULL UNIQUE REFERENCES mobile_plan_steps(id),
 device_id TEXT NOT NULL,
 session_id TEXT NOT NULL REFERENCES mobile_device_sessions(id),
 planned_observation_id TEXT REFERENCES mobile_observations(id),
 acquired_at TEXT NOT NULL,
 completed_at TEXT,
 status TEXT NOT NULL CHECK(status IN ('claimed','verified','failed','interrupted'))
);
CREATE TRIGGER mobile_executor_no_delete BEFORE DELETE ON mobile_executor_attempts BEGIN SELECT RAISE(ABORT,'execution history retained'); END;
CREATE TRIGGER mobile_executor_identity_immutable BEFORE UPDATE ON mobile_executor_attempts
WHEN OLD.id<>NEW.id OR OLD.goal_id<>NEW.goal_id OR OLD.plan_id<>NEW.plan_id OR OLD.step_id<>NEW.step_id OR OLD.device_id<>NEW.device_id OR OLD.session_id<>NEW.session_id OR OLD.planned_observation_id IS NOT NEW.planned_observation_id OR OLD.acquired_at<>NEW.acquired_at OR OLD.status<>'claimed'
BEGIN SELECT RAISE(ABORT,'execution identity retained'); END;
CREATE TABLE mobile_extracted_objects (
 id TEXT PRIMARY KEY,
 step_id TEXT NOT NULL UNIQUE REFERENCES mobile_plan_steps(id),
 observation_id TEXT NOT NULL REFERENCES mobile_observations(id),
 evidence_id TEXT NOT NULL UNIQUE REFERENCES evidence(id),
 source_id TEXT NOT NULL REFERENCES sources(id),
 captured_at TEXT NOT NULL,
 domain_json TEXT NOT NULL CHECK(json_valid(domain_json))
);
CREATE TRIGGER mobile_extracted_no_update BEFORE UPDATE ON mobile_extracted_objects BEGIN SELECT RAISE(ABORT,'extracted objects immutable'); END;
CREATE TRIGGER mobile_extracted_no_delete BEFORE DELETE ON mobile_extracted_objects BEGIN SELECT RAISE(ABORT,'extracted objects retained'); END;
-- User-bound typed completion support, never inferred from model prose.
CREATE TABLE mobile_completion_criteria (
 goal_id TEXT PRIMARY KEY REFERENCES mobile_goals(id),
 step_id TEXT NOT NULL UNIQUE REFERENCES mobile_plan_steps(id),
 objective TEXT NOT NULL,
 expected_json TEXT NOT NULL CHECK(json_valid(expected_json))
);
CREATE TRIGGER mobile_completion_no_update BEFORE UPDATE ON mobile_completion_criteria BEGIN SELECT RAISE(ABORT,'completion support immutable'); END;
CREATE TRIGGER mobile_completion_no_delete BEFORE DELETE ON mobile_completion_criteria BEGIN SELECT RAISE(ABORT,'completion support retained'); END;
