use crate::{AppError, AppRuntime, events::append_event};
use mobile_runtime::collection::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use sqlx::{Row, SqlitePool};
#[derive(Clone, Debug, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCollectionWorkspace {
    pub observations: Vec<MobileObservation>,
    pub diffs: Vec<ObservationDiff>,
    pub actions: Vec<MobileActionResult>,
    pub data_objects: Vec<MobileDataObject>,
    pub last_extraction_count: usize,
    pub last_new_object_count: usize,
}

/// Typed views over the existing SQLite pool; no second database or cloud writer.
pub struct LocalMobileRepository<'a, T> {
    pool: &'a SqlitePool,
    table: &'static str,
    marker: std::marker::PhantomData<T>,
}
impl<'a, T: DeserializeOwned> LocalMobileRepository<'a, T> {
    fn new(pool: &'a SqlitePool, table: &'static str) -> Self {
        Self {
            pool,
            table,
            marker: std::marker::PhantomData,
        }
    }
    async fn query(
        &self,
        field: Option<&str>,
        value: Option<&str>,
        limit: u32,
    ) -> Result<Vec<T>, AppError> {
        let condition = field.map(|f| format!(" WHERE {f}=?")).unwrap_or_default();
        let statement = format!(
            "SELECT domain_json FROM {}{} ORDER BY captured_at DESC,id DESC LIMIT ?",
            self.table, condition
        );
        let mut q = sqlx::query_scalar::<_, String>(&statement);
        if let Some(v) = value {
            q = q.bind(v);
        }
        q.bind(limit.min(500))
            .fetch_all(self.pool)
            .await?
            .iter()
            .map(|j| Ok(serde_json::from_str(j)?))
            .collect()
    }
    pub async fn get_by_id(&self, id: &str) -> Result<Option<T>, AppError> {
        Ok(self
            .query(Some("id"), Some(id), 1)
            .await?
            .into_iter()
            .next())
    }
    pub async fn get_recent(&self, limit: u32) -> Result<Vec<T>, AppError> {
        self.query(None, None, limit).await
    }
    pub async fn get_by_device(&self, id: &str) -> Result<Vec<T>, AppError> {
        self.query(Some("device_id"), Some(id), 500).await
    }
    pub async fn get_by_package(&self, package: &str) -> Result<Vec<T>, AppError> {
        self.query(Some("package_name"), Some(package), 500).await
    }
    pub async fn get_by_observation(&self, id: &str) -> Result<Vec<T>, AppError> {
        if self.table == "mobile_data_objects" {
            let rows:Vec<String>=sqlx::query_scalar("SELECT DISTINCT o.domain_json FROM mobile_data_objects o JOIN mobile_data_object_sightings s ON s.object_id=o.id WHERE s.observation_id=? LIMIT 500").bind(id).fetch_all(self.pool).await?;
            return rows.iter().map(|j| Ok(serde_json::from_str(j)?)).collect();
        }
        if self.table == "mobile_interaction_actions" {
            let rows:Vec<String>=sqlx::query_scalar("SELECT domain_json FROM mobile_interaction_actions WHERE observation_id=? OR after_observation_id=? ORDER BY captured_at DESC LIMIT 500").bind(id).bind(id).fetch_all(self.pool).await?;
            return rows.iter().map(|j| Ok(serde_json::from_str(j)?)).collect();
        }
        self.query(Some("observation_id"), Some(id), 500).await
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileDataProvenance {
    pub object: MobileDataObject,
    pub observation: MobileObservation,
    pub extracted_data: Vec<ExtractedMobileData>,
    pub sighting_count: i64,
}
pub type MobileObservationRepository<'a> = LocalMobileRepository<'a, MobileObservation>;
pub type MobileActionRepository<'a> = LocalMobileRepository<'a, MobileActionResult>;
pub type ObservationDiffRepository<'a> = LocalMobileRepository<'a, ObservationDiff>;
pub type MobileDataRepository<'a> = LocalMobileRepository<'a, MobileDataObject>;
pub type MobileExtractedDataRepository<'a> = LocalMobileRepository<'a, ExtractedMobileData>;
impl AppRuntime {
    pub fn mobile_observation_repository(&self) -> MobileObservationRepository<'_> {
        LocalMobileRepository::new(self.pool(), "mobile_acquisition_observations")
    }
    pub fn mobile_action_repository(&self) -> MobileActionRepository<'_> {
        LocalMobileRepository::new(self.pool(), "mobile_interaction_actions")
    }
    pub fn observation_diff_repository(&self) -> ObservationDiffRepository<'_> {
        LocalMobileRepository::new(self.pool(), "mobile_observation_diffs")
    }
    pub fn mobile_data_repository(&self) -> MobileDataRepository<'_> {
        LocalMobileRepository::new(self.pool(), "mobile_data_objects")
    }
    pub fn mobile_extracted_data_repository(&self) -> MobileExtractedDataRepository<'_> {
        LocalMobileRepository::new(self.pool(), "mobile_extracted_data")
    }
    pub async fn collect_mobile_observation(
        &self,
        observation: &MobileObservation,
    ) -> Result<(), AppError> {
        if observation.source != "android_ui_tree"
            || observation.element_count != observation.elements.len()
            || observation.elements.iter().any(|e| {
                (e.redacted
                    && (e.text.is_some()
                        || e.content_description.is_some()
                        || e.resource_id.is_some()))
                    || (!e.redacted
                        && sensitive_label(&format!(
                            "{} {} {}",
                            e.text.as_deref().unwrap_or_default(),
                            e.content_description.as_deref().unwrap_or_default(),
                            e.resource_id.as_deref().unwrap_or_default()
                        )))
            })
        {
            return Err(AppError::InvalidData);
        }
        let settings = self.load_mobile_settings().await?;
        let mut source = crate::mobile_executor::mobile_source_definition();
        source.id = format!("android-manual-{}", observation.package_name);
        source.name = format!("Manual Android UI: {}", observation.package_name);
        source.endpoint = format!("android://{}", observation.package_name);
        source.enabled = false;
        let mut registry = collector_runtime::SourceRegistry::default();
        registry
            .register_manual_mobile(source.clone(), &settings.allowed_apps)
            .map_err(|_| AppError::InvalidData)?;
        let mut tx = self.pool().begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("INSERT INTO sources(id,name,endpoint,collector_type,classification,capabilities_json,reliability_tier,poll_policy_json,rate_limit_json,auth_requirement,retention_policy_json,enabled,created_at,updated_at) VALUES (?,?,?,'computer','company',?,'tier4_unverified',?,?,'none',?,0,?,?) ON CONFLICT(id) DO NOTHING")
            .bind(&source.id).bind(&source.name).bind(&source.endpoint).bind(serde_json::to_string(&source.capabilities)?).bind(serde_json::to_string(&source.poll)?).bind(serde_json::to_string(&source.rate_limit)?).bind(serde_json::to_string(&source.retention)?).bind(observation.captured_at.to_rfc3339()).bind(observation.captured_at.to_rfc3339()).execute(&mut *tx).await?;
        let inserted=sqlx::query("INSERT INTO mobile_acquisition_observations(id,source_id,device_id,package_name,activity_name,observation_id,captured_at,previous_observation_id,domain_json) VALUES (?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO NOTHING")
            .bind(&observation.id).bind(&source.id).bind(&observation.device_id).bind(&observation.package_name).bind(&observation.activity_name).bind(&observation.id).bind(observation.captured_at.to_rfc3339()).bind(&observation.previous_observation_id).bind(serde_json::to_string(observation)?).execute(&mut *tx).await?.rows_affected();
        if inserted == 0 {
            tx.commit().await?;
            return Ok(());
        }
        let mut events=vec![append_event(&mut tx,"mobile.acquisition_observation","mobile_acquisition",&observation.device_id,None,None,Some(&observation.id),&json!({"triggeredBy":"owner","deviceId":observation.device_id,"observationId":observation.id,"elementCount":observation.element_count,"redactions":observation.redactions})).await?];
        if let Some(before_id) = &observation.previous_observation_id {
            let j: String = sqlx::query_scalar(
                "SELECT domain_json FROM mobile_acquisition_observations WHERE id=?",
            )
            .bind(before_id)
            .fetch_one(&mut *tx)
            .await?;
            let before: MobileObservation = serde_json::from_str(&j)?;
            if before.device_id != observation.device_id {
                return Err(AppError::InvalidData);
            }
            let diff = diff_observations(&before, observation);
            sqlx::query("INSERT INTO mobile_observation_diffs(id,device_id,package_name,observation_id,before_observation_id,captured_at,domain_json) VALUES (?,?,?,?,?,?,?)").bind(&observation.id).bind(&observation.device_id).bind(&observation.package_name).bind(&observation.id).bind(before_id).bind(observation.captured_at.to_rfc3339()).bind(serde_json::to_string(&diff)?).execute(&mut *tx).await?;
            events.push(
                append_event(
                    &mut tx,
                    "mobile.observation_diff",
                    "mobile_acquisition",
                    &observation.device_id,
                    None,
                    None,
                    Some(&observation.id),
                    &json!({"triggeredBy":"owner","deviceId":observation.device_id,"diff":diff}),
                )
                .await?,
            );
        }
        tx.commit().await?;
        for event in events {
            self.event_bus.publish(event);
        }
        Ok(())
    }
    pub async fn record_mobile_interaction(
        &self,
        result: &MobileActionResult,
        package: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool().begin_with("BEGIN IMMEDIATE").await?;
        let previous: Option<String> =
            sqlx::query_scalar("SELECT status FROM mobile_interaction_actions WHERE id=?")
                .bind(&result.action_id)
                .fetch_optional(&mut *tx)
                .await?;
        if previous
            .as_deref()
            .is_some_and(|s| s != "pending" && s != "running")
        {
            return Err(AppError::InvalidData);
        }
        sqlx::query("INSERT INTO mobile_interaction_actions(id,device_id,package_name,observation_id,after_observation_id,captured_at,status,domain_json) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET device_id=excluded.device_id,package_name=excluded.package_name,observation_id=excluded.observation_id,after_observation_id=excluded.after_observation_id,status=excluded.status,domain_json=excluded.domain_json")
            .bind(&result.action_id).bind(&result.device_id).bind(package).bind(&result.before_observation_id).bind(&result.after_observation_id).bind(result.started_at.to_rfc3339()).bind(&result.status).bind(serde_json::to_string(result)?).execute(&mut *tx).await?;
        if result.status != "running"
            && result.status != "pending"
            && let (Some(before_id), Some(after_id)) =
                (&result.before_observation_id, &result.after_observation_id)
            && before_id != after_id
        {
            let mut observations = Vec::new();
            for id in [before_id, after_id] {
                let j: String = sqlx::query_scalar(
                    "SELECT domain_json FROM mobile_acquisition_observations WHERE id=?",
                )
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
                observations.push(serde_json::from_str::<MobileObservation>(&j)?);
            }
            if observations.iter().any(|o| o.device_id != result.device_id) {
                return Err(AppError::InvalidData);
            }
            let diff = diff_observations(&observations[0], &observations[1]);
            sqlx::query("INSERT INTO mobile_observation_diffs(id,device_id,package_name,observation_id,before_observation_id,captured_at,domain_json) VALUES (?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET before_observation_id=excluded.before_observation_id,domain_json=excluded.domain_json")
                .bind(after_id).bind(&result.device_id).bind(&observations[1].package_name).bind(after_id).bind(before_id).bind(observations[1].captured_at.to_rfc3339()).bind(serde_json::to_string(&diff)?).execute(&mut *tx).await?;
            append_event(&mut tx,"mobile.action_diff","mobile_acquisition",&result.device_id,None,None,Some(after_id),&json!({"triggeredBy":"owner","actionId":result.action_id,"deviceId":result.device_id,"diff":diff})).await?;
        }
        let event = append_event(
            &mut tx,
            "mobile.manual_action",
            "mobile_acquisition",
            &result.device_id,
            None,
            None,
            Some(&result.action_id),
            &serde_json::to_value(result)?,
        )
        .await?;
        tx.commit().await?;
        self.event_bus.publish(event);
        Ok(())
    }
    pub async fn record_mobile_manual_receipt(
        &self,
        receipt: &mobile_runtime::MobileActionReceipt,
        device_id: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool().begin().await?;
        let event = append_event(
            &mut tx,
            "mobile.manual_action_receipt",
            "mobile_acquisition",
            device_id,
            None,
            None,
            Some(&receipt.action_id),
            &json!({"triggeredBy":"owner","deviceId":device_id,"receipt":receipt}),
        )
        .await?;
        tx.commit().await?;
        self.event_bus.publish(event);
        Ok(())
    }
    pub async fn extract_mobile_observation(
        &self,
        id: &str,
    ) -> Result<MobileCollectionWorkspace, AppError> {
        let observation = self
            .mobile_observation_repository()
            .get_by_id(id)
            .await?
            .ok_or(AppError::NotFound("observation"))?;
        let mut data = extract_mobile_data(&observation);
        let mut tx = self.pool().begin_with("BEGIN IMMEDIATE").await?;
        for d in &mut data {
            let key = format!(
                "{}:{}:{}:{}",
                d.observation_id, d.element_id, d.data_type, d.value
            );
            sqlx::query("INSERT INTO mobile_extracted_data(id,device_id,package_name,observation_id,element_id,captured_at,deduplication_key,domain_json) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(deduplication_key) DO NOTHING")
                .bind(&d.id).bind(&observation.device_id).bind(&observation.package_name).bind(id).bind(&d.element_id).bind(d.captured_at.to_rfc3339()).bind(&key).bind(serde_json::to_string(d)?).execute(&mut *tx).await?;
            d.id = sqlx::query_scalar(
                "SELECT id FROM mobile_extracted_data WHERE deduplication_key=?",
            )
            .bind(key)
            .fetch_one(&mut *tx)
            .await?;
        }
        let objects = build_data_objects(&observation, &data);
        let mut new_count = 0;
        for object in &objects {
            new_count+=sqlx::query("INSERT INTO mobile_data_objects(id,device_id,package_name,observation_id,captured_at,deduplication_key,domain_json) VALUES (?,?,?,?,?,?,?) ON CONFLICT(deduplication_key) DO NOTHING")
                .bind(&object.id).bind(&object.device_id).bind(&object.package_name).bind(id).bind(object.captured_at.to_rfc3339()).bind(&object.deduplication_key).bind(serde_json::to_string(object)?).execute(&mut *tx).await?.rows_affected() as usize;
            let object_id: String =
                sqlx::query_scalar("SELECT id FROM mobile_data_objects WHERE deduplication_key=?")
                    .bind(&object.deduplication_key)
                    .fetch_one(&mut *tx)
                    .await?;
            for d in data
                .iter()
                .filter(|d| object.provenance.element_ids.contains(&d.element_id))
            {
                sqlx::query("INSERT OR IGNORE INTO mobile_data_object_sightings(object_id,observation_id,element_id,extracted_data_id) VALUES (?,?,?,?)").bind(&object_id).bind(id).bind(&d.element_id).bind(&d.id).execute(&mut *tx).await?;
            }
        }
        sqlx::query("INSERT INTO mobile_extraction_runs(id,observation_id,captured_at,extracted_count,new_object_count) VALUES (?,?,?,?,?)").bind(format!("extraction_{}",uuid::Uuid::now_v7())).bind(id).bind(chrono::Utc::now().to_rfc3339()).bind(data.len() as i64).bind(new_count as i64).execute(&mut *tx).await?;
        let event=append_event(&mut tx,"mobile.data_extracted","mobile_acquisition",&observation.device_id,None,None,Some(id),&json!({"triggeredBy":"owner","deviceId":observation.device_id,"observationId":id,"extractedCount":data.len(),"newObjectCount":new_count,"localOnly":true})).await?;
        tx.commit().await?;
        self.event_bus.publish(event);
        self.mobile_collection_workspace().await
    }
    pub async fn mobile_data_provenance(&self, id: &str) -> Result<MobileDataProvenance, AppError> {
        let object = self
            .mobile_data_repository()
            .get_by_id(id)
            .await?
            .ok_or(AppError::NotFound("data object"))?;
        let observation = self
            .mobile_observation_repository()
            .get_by_id(&object.observation_id)
            .await?
            .ok_or(AppError::NotFound("observation"))?;
        let rows:Vec<String>=sqlx::query_scalar("SELECT DISTINCT d.domain_json FROM mobile_extracted_data d JOIN mobile_data_object_sightings s ON s.extracted_data_id=d.id WHERE s.object_id=? AND s.observation_id=?").bind(id).bind(&object.observation_id).fetch_all(self.pool()).await?;
        let extracted_data = rows
            .iter()
            .map(|j| serde_json::from_str(j))
            .collect::<Result<Vec<_>, _>>()?;
        let sighting_count=sqlx::query_scalar("SELECT COUNT(DISTINCT observation_id) FROM mobile_data_object_sightings WHERE object_id=?").bind(id).fetch_one(self.pool()).await?;
        Ok(MobileDataProvenance {
            object,
            observation,
            extracted_data,
            sighting_count,
        })
    }
    pub async fn recover_mobile_interactions(&self) -> Result<(), AppError> {
        let rows:Vec<String>=sqlx::query_scalar("SELECT domain_json FROM mobile_interaction_actions WHERE status IN ('pending','running')").fetch_all(self.pool()).await?;
        let pending = rows
            .iter()
            .map(|j| serde_json::from_str::<MobileActionResult>(j))
            .collect::<Result<Vec<_>, _>>()?;
        for mut result in pending {
            result.status = "failed".into();
            result.completed_at = Some(chrono::Utc::now());
            result.error_code = Some("INTERRUPTED_BY_RESTART".into());
            result.error_message = Some("INTERRUPTED_BY_RESTART".into());
            let package: String = sqlx::query_scalar(
                "SELECT package_name FROM mobile_interaction_actions WHERE id=?",
            )
            .bind(&result.action_id)
            .fetch_one(self.pool())
            .await?;
            self.record_mobile_interaction(&result, &package).await?;
        }
        Ok(())
    }
    pub async fn mobile_collection_workspace(&self) -> Result<MobileCollectionWorkspace, AppError> {
        let last=sqlx::query("SELECT extracted_count,new_object_count FROM mobile_extraction_runs ORDER BY captured_at DESC,id DESC LIMIT 1").fetch_optional(self.pool()).await?;
        Ok(MobileCollectionWorkspace {
            observations: self.mobile_observation_repository().get_recent(100).await?,
            diffs: self.observation_diff_repository().get_recent(100).await?,
            actions: self.mobile_action_repository().get_recent(100).await?,
            data_objects: self.mobile_data_repository().get_recent(200).await?,
            last_extraction_count: last
                .as_ref()
                .map(|r| r.get::<i64, _>("extracted_count") as usize)
                .unwrap_or_default(),
            last_new_object_count: last
                .as_ref()
                .map(|r| r.get::<i64, _>("new_object_count") as usize)
                .unwrap_or_default(),
        })
    }
}
#[cfg(test)]
mod tests;
