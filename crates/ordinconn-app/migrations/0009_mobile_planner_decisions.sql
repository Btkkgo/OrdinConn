-- Planner proposals and metadata only. No execution or production provider creation.
CREATE TABLE mobile_planner_decisions (
  id TEXT PRIMARY KEY,
  goal_id TEXT NOT NULL REFERENCES mobile_goals(id) ON DELETE RESTRICT,
  plan_id TEXT REFERENCES mobile_plans(id) ON DELETE RESTRICT,
  step_id TEXT REFERENCES mobile_plan_steps(id) ON DELETE RESTRICT,
  observation_id TEXT REFERENCES mobile_observations(id) ON DELETE RESTRICT,
  planner_version TEXT NOT NULL,
  schema_version TEXT NOT NULL,
  model_provider_id TEXT NOT NULL,
  model_name TEXT NOT NULL,
  created_at TEXT NOT NULL,
  outcome_json TEXT NOT NULL CHECK(json_valid(outcome_json))
);
CREATE INDEX mobile_planner_goal_history ON mobile_planner_decisions(goal_id,created_at);
CREATE TRIGGER mobile_planner_no_delete BEFORE DELETE ON mobile_planner_decisions BEGIN SELECT RAISE(ABORT,'planner history retained'); END;
CREATE TRIGGER mobile_planner_no_update BEFORE UPDATE ON mobile_planner_decisions BEGIN SELECT RAISE(ABORT,'planner decisions immutable'); END;
