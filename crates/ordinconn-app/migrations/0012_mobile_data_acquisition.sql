-- Additive local-only acquisition storage in the existing application database.
CREATE TABLE mobile_acquisition_observations (
 id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES sources(id), device_id TEXT NOT NULL, package_name TEXT NOT NULL, activity_name TEXT NOT NULL,
 observation_id TEXT NOT NULL UNIQUE, captured_at TEXT NOT NULL, previous_observation_id TEXT REFERENCES mobile_acquisition_observations(id),
 domain_json TEXT NOT NULL CHECK(json_valid(domain_json))
);
CREATE INDEX mobile_acquisition_device_time ON mobile_acquisition_observations(device_id,captured_at);
CREATE INDEX mobile_acquisition_package_time ON mobile_acquisition_observations(package_name,captured_at);
CREATE TABLE mobile_interaction_actions (
 id TEXT PRIMARY KEY, device_id TEXT NOT NULL, package_name TEXT NOT NULL, observation_id TEXT REFERENCES mobile_acquisition_observations(id),
 after_observation_id TEXT REFERENCES mobile_acquisition_observations(id), captured_at TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('pending','running','completed','failed','cancelled')),
 domain_json TEXT NOT NULL CHECK(json_valid(domain_json))
);
CREATE TABLE mobile_observation_diffs (
 id TEXT PRIMARY KEY REFERENCES mobile_acquisition_observations(id), device_id TEXT NOT NULL, package_name TEXT NOT NULL,
 observation_id TEXT NOT NULL REFERENCES mobile_acquisition_observations(id), before_observation_id TEXT NOT NULL REFERENCES mobile_acquisition_observations(id),
 captured_at TEXT NOT NULL, domain_json TEXT NOT NULL CHECK(json_valid(domain_json))
);
CREATE TABLE mobile_extracted_data (
 id TEXT PRIMARY KEY, device_id TEXT NOT NULL, package_name TEXT NOT NULL,
 observation_id TEXT NOT NULL REFERENCES mobile_acquisition_observations(id), element_id TEXT NOT NULL,
 captured_at TEXT NOT NULL, deduplication_key TEXT NOT NULL UNIQUE, domain_json TEXT NOT NULL CHECK(json_valid(domain_json))
);
CREATE TABLE mobile_data_objects (
 id TEXT PRIMARY KEY, device_id TEXT NOT NULL, package_name TEXT NOT NULL,
 observation_id TEXT NOT NULL REFERENCES mobile_acquisition_observations(id), captured_at TEXT NOT NULL,
 deduplication_key TEXT NOT NULL UNIQUE, domain_json TEXT NOT NULL CHECK(json_valid(domain_json))
);
CREATE TABLE mobile_data_object_sightings (
 object_id TEXT NOT NULL REFERENCES mobile_data_objects(id), observation_id TEXT NOT NULL REFERENCES mobile_acquisition_observations(id),
 element_id TEXT NOT NULL, extracted_data_id TEXT NOT NULL REFERENCES mobile_extracted_data(id),
 PRIMARY KEY(object_id,observation_id,element_id,extracted_data_id)
);
CREATE TABLE mobile_extraction_runs (
 id TEXT PRIMARY KEY, observation_id TEXT NOT NULL REFERENCES mobile_acquisition_observations(id), captured_at TEXT NOT NULL,
 extracted_count INTEGER NOT NULL, new_object_count INTEGER NOT NULL
);
CREATE INDEX mobile_extracted_observation ON mobile_extracted_data(observation_id);
CREATE INDEX mobile_objects_observation ON mobile_data_objects(observation_id);
