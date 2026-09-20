use chrono::Utc;
use mobile_runtime::{
    MobileCapture, MobileDeviceSession, MobileFrame, MobileObservation, MobileUiSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{AppError, AppRuntime, events::append_event};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileResearchBudget {
    pub max_duration_seconds: u64,
    pub max_steps: u32,
    pub max_scrolls: u32,
    pub max_pages: u32,
    pub max_observations: u32,
    pub max_model_calls: u32,
}

impl Default for MobileResearchBudget {
    fn default() -> Self {
        Self {
            max_duration_seconds: 300,
            max_steps: 40,
            max_scrolls: 12,
            max_pages: 20,
            max_observations: 50,
            max_model_calls: 10,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileRuntimeSettings {
    pub android_sdk: Option<String>,
    pub allowed_apps: Vec<String>,
    pub screenshot_retention: String,
    pub research_budget: MobileResearchBudget,
    pub text_scale: u16,
}

impl Default for MobileRuntimeSettings {
    fn default() -> Self {
        Self {
            android_sdk: None,
            allowed_apps: Vec::new(),
            screenshot_retention: "memory_only".into(),
            research_budget: MobileResearchBudget::default(),
            text_scale: 100,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceItemView {
    pub id: String,
    pub source_method: String,
    pub source_app: String,
    pub source_account: Option<String>,
    pub title: String,
    pub summary: String,
    pub observed_at: String,
    pub data_type: String,
    pub evidence_status: String,
    pub evidence_quality: Option<f64>,
    pub confidence: f64,
    pub assets: Vec<String>,
    pub favorite: bool,
    pub saved: bool,
    pub official_source: bool,
    pub has_contradiction: bool,
    pub mobile_observation_id: Option<String>,
    pub evidence_ids: Vec<String>,
    pub related_signal_ids: Vec<String>,
    pub source_locator: Option<String>,
    pub visible_facts: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarehouseEntryView {
    pub id: String,
    pub item_id: String,
    pub favorite: bool,
    pub saved: bool,
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub collection: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyDefinitionView {
    pub id: String,
    pub version: String,
    pub market: String,
    pub category: String,
    pub enabled: bool,
    pub required_inputs: Vec<String>,
    pub readiness: String,
    pub parameters: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileResearchTaskView {
    pub id: String,
    pub query: String,
    pub status: String,
    pub allowed_apps: Vec<String>,
    pub budget: MobileResearchBudget,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileWorkspaceData {
    pub runtime_status: String,
    pub adb_status: String,
    pub session: Option<MobileDeviceSession>,
    pub ui_snapshot: Option<MobileUiSnapshot>,
    pub frame: Option<MobileFrame>,
    pub observations: Vec<MobileObservation>,
    pub feed: Vec<IntelligenceItemView>,
    pub warehouse: Vec<WarehouseEntryView>,
    pub strategies: Vec<StrategyDefinitionView>,
    pub settings: MobileRuntimeSettings,
}

impl AppRuntime {
    pub async fn record_mobile_capture(&self, capture: &MobileCapture) -> Result<(), AppError> {
        let session = &capture.session;
        let snapshot = &capture.snapshot;
        let observation = &capture.observation;
        let session_json = serde_json::to_string(session)?;
        let snapshot_json = serde_json::to_string(snapshot)?;
        let observation_json = serde_json::to_string(observation)?;
        let mut transaction = self.pool().begin().await?;
        sqlx::query("INSERT INTO mobile_device_sessions (id,device_id,status,current_app,current_activity,connected_at,last_observation_at,domain_json) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET status=excluded.status,current_app=excluded.current_app,current_activity=excluded.current_activity,last_observation_at=excluded.last_observation_at,domain_json=excluded.domain_json")
            .bind(&session.session_id)
            .bind(&session.device_id)
            .bind(enum_value(&session.status))
            .bind(&session.current_app)
            .bind(&session.current_activity)
            .bind(session.connected_at.to_rfc3339())
            .bind(session.last_observation_at.map(|value| value.to_rfc3339()))
            .bind(session_json)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("INSERT INTO mobile_ui_snapshots (id,session_id,package_name,activity,ui_tree_hash,element_count,captured_at,domain_json) VALUES (?,?,?,?,?,?,?,?)")
            .bind(&snapshot.snapshot_id)
            .bind(&snapshot.session_id)
            .bind(&snapshot.package_name)
            .bind(&snapshot.activity)
            .bind(&observation.ui_tree_hash)
            .bind(snapshot.elements.len() as i64)
            .bind(snapshot.captured_at.to_rfc3339())
            .bind(snapshot_json)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("INSERT INTO mobile_observations (id,task_id,session_id,snapshot_id,package_name,observed_at,frame_hash,ui_tree_hash,evidence_status,domain_json) VALUES (?,?,?,?,?,?,?,?,?,?)")
            .bind(&observation.id)
            .bind(&observation.task_id)
            .bind(&observation.device_session_id)
            .bind(&snapshot.snapshot_id)
            .bind(&observation.package_name)
            .bind(observation.observed_at.to_rfc3339())
            .bind(&observation.frame_hash)
            .bind(&observation.ui_tree_hash)
            .bind(enum_value(&observation.evidence_status))
            .bind(observation_json)
            .execute(&mut *transaction)
            .await?;
        let events = [
            append_event(
                &mut transaction,
                "mobile.session_started",
                "mobile_session",
                &session.session_id,
                None,
                None,
                Some(&session.session_id),
                &json!({"sessionId": session.session_id, "deviceId": session.device_id}),
            )
            .await?,
            append_event(
                &mut transaction,
                "mobile.snapshot",
                "mobile_session",
                &session.session_id,
                None,
                None,
                Some(&snapshot.snapshot_id),
                &json!({"snapshotId": snapshot.snapshot_id, "frameHash": observation.frame_hash, "uiTreeHash": observation.ui_tree_hash, "elementCount": snapshot.elements.len()}),
            )
            .await?,
            append_event(
                &mut transaction,
                "mobile.observation",
                "mobile_session",
                &session.session_id,
                None,
                None,
                Some(&observation.id),
                &json!({"observationId": observation.id, "entityCount": observation.extracted_entities.len(), "redactionCount": observation.redactions.len()}),
            )
            .await?,
        ];
        transaction.commit().await?;
        for event in events {
            self.event_bus.publish(event);
        }
        Ok(())
    }

    pub async fn set_warehouse_entry(
        &self,
        item_id: &str,
        favorite: bool,
        saved: bool,
        tags: Vec<String>,
    ) -> Result<WarehouseEntryView, AppError> {
        let now = Utc::now().to_rfc3339();
        let id = format!("warehouse_{}", Uuid::now_v7());
        sqlx::query("INSERT INTO warehouse_entries (id,item_id,favorite,saved,tags_json,created_at,updated_at) VALUES (?,?,?,?,?,?,?) ON CONFLICT(item_id) DO UPDATE SET favorite=excluded.favorite,saved=excluded.saved,tags_json=excluded.tags_json,updated_at=excluded.updated_at")
            .bind(&id)
            .bind(item_id)
            .bind(favorite)
            .bind(saved)
            .bind(serde_json::to_string(&tags)?)
            .bind(&now)
            .bind(&now)
            .execute(self.pool())
            .await?;
        self.warehouse_entry(item_id).await
    }

    pub async fn save_mobile_settings(
        &self,
        settings: &MobileRuntimeSettings,
    ) -> Result<(), AppError> {
        if settings.screenshot_retention != "memory_only"
            || !matches!(settings.text_scale, 90 | 100 | 110 | 120)
        {
            return Err(AppError::InvalidData);
        }
        sqlx::query("INSERT INTO settings (key,value_json,updated_at) VALUES ('mobile_runtime',?,?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at")
            .bind(serde_json::to_string(settings)?)
            .bind(Utc::now().to_rfc3339())
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn create_mobile_research_task(
        &self,
        query: &str,
        allowed_apps: Vec<String>,
        budget: MobileResearchBudget,
    ) -> Result<MobileResearchTaskView, AppError> {
        if query.trim().is_empty() || allowed_apps.is_empty() {
            return Err(AppError::InvalidData);
        }
        let id = format!("research_task_{}", Uuid::now_v7());
        let created_at = Utc::now().to_rfc3339();
        sqlx::query("INSERT INTO research_tasks (id,query,reason,status,discovered_urls_json,created_at,mobile_packages_json,mobile_budget_json) VALUES (?,?,?,'pending','[]',?,?,?)")
            .bind(&id)
            .bind(query.trim())
            .bind("User-authorized mobile intelligence research")
            .bind(&created_at)
            .bind(serde_json::to_string(&allowed_apps)?)
            .bind(serde_json::to_string(&budget)?)
            .execute(self.pool())
            .await?;
        Ok(MobileResearchTaskView {
            id,
            query: query.trim().into(),
            status: "pending".into(),
            allowed_apps,
            budget,
            created_at,
        })
    }

    pub async fn set_strategy_enabled(
        &self,
        strategy_id: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE strategy_definitions SET enabled=? WHERE id=?")
            .bind(enabled)
            .bind(strategy_id)
            .execute(self.pool())
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("strategy"));
        }
        Ok(())
    }

    pub async fn mobile_workspace_data(&self) -> Result<MobileWorkspaceData, AppError> {
        let settings = self.load_mobile_settings().await?;
        let observations = self.mobile_observations().await?;
        let warehouse = self.warehouse_entries().await?;
        let warehouse_by_item: HashMap<_, _> = warehouse
            .iter()
            .map(|entry| (entry.item_id.clone(), entry))
            .collect();
        let mut feed = Vec::new();
        for observation in &observations {
            let stored = warehouse_by_item.get(&observation.id);
            let related_signal_ids =
                related_signals(self.pool(), &observation.extracted_entities).await?;
            feed.push(IntelligenceItemView {
                id: observation.id.clone(),
                source_method: "MOBILE".into(),
                source_app: observation.package_name.clone(),
                source_account: observation.author.clone(),
                title: observation
                    .visible_facts
                    .first()
                    .cloned()
                    .unwrap_or_else(|| observation.package_name.clone()),
                summary: observation.visible_facts.join(" · "),
                observed_at: observation.observed_at.to_rfc3339(),
                data_type: "mobile_observation".into(),
                evidence_status: enum_value(&observation.evidence_status),
                evidence_quality: None,
                confidence: observation.extraction_confidence,
                assets: observation.extracted_entities.clone(),
                favorite: stored.is_some_and(|entry| entry.favorite),
                saved: stored.is_some_and(|entry| entry.saved),
                official_source: false,
                has_contradiction: false,
                mobile_observation_id: Some(observation.id.clone()),
                evidence_ids: Vec::new(),
                related_signal_ids,
                source_locator: Some(observation.source_locator.clone()),
                visible_facts: observation.visible_facts.clone(),
            });
        }
        feed.extend(self.evidence_feed(&warehouse_by_item).await?);
        feed.sort_by(|left, right| right.observed_at.cmp(&left.observed_at));
        Ok(MobileWorkspaceData {
            runtime_status: if observations.is_empty() {
                "idle"
            } else {
                "connected"
            }
            .into(),
            adb_status: "missing".into(),
            session: None,
            ui_snapshot: None,
            frame: None,
            observations,
            feed,
            warehouse,
            strategies: self.strategy_views().await?,
            settings,
        })
    }

    async fn load_mobile_settings(&self) -> Result<MobileRuntimeSettings, AppError> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value_json FROM settings WHERE key='mobile_runtime'")
                .fetch_optional(self.pool())
                .await?;
        value
            .map(|value| serde_json::from_str(&value).map_err(AppError::from))
            .transpose()
            .map(|settings| settings.unwrap_or_default())
    }

    async fn mobile_observations(&self) -> Result<Vec<MobileObservation>, AppError> {
        let values: Vec<String> = sqlx::query_scalar(
            "SELECT domain_json FROM mobile_observations ORDER BY observed_at DESC LIMIT 200",
        )
        .fetch_all(self.pool())
        .await?;
        values
            .into_iter()
            .map(|value| serde_json::from_str(&value).map_err(AppError::from))
            .collect()
    }

    async fn warehouse_entry(&self, item_id: &str) -> Result<WarehouseEntryView, AppError> {
        let row = sqlx::query("SELECT id,item_id,favorite,saved,tags_json,note,collection_name,created_at,updated_at FROM warehouse_entries WHERE item_id=?")
            .bind(item_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or(AppError::NotFound("warehouse entry"))?;
        warehouse_from_row(&row)
    }

    async fn warehouse_entries(&self) -> Result<Vec<WarehouseEntryView>, AppError> {
        let rows = sqlx::query("SELECT id,item_id,favorite,saved,tags_json,note,collection_name,created_at,updated_at FROM warehouse_entries ORDER BY updated_at DESC")
            .fetch_all(self.pool())
            .await?;
        rows.iter().map(warehouse_from_row).collect()
    }

    async fn strategy_views(&self) -> Result<Vec<StrategyDefinitionView>, AppError> {
        let rows = sqlx::query("SELECT id,version,market,category,enabled,required_observations_json,parameters_json FROM strategy_definitions ORDER BY category,id")
            .fetch_all(self.pool())
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(StrategyDefinitionView {
                    id: row.get("id"),
                    version: row.get("version"),
                    market: row.get("market"),
                    category: row.get("category"),
                    enabled: row.get("enabled"),
                    required_inputs: serde_json::from_str(
                        &row.get::<String, _>("required_observations_json"),
                    )?,
                    readiness: "configured".into(),
                    parameters: serde_json::from_str(&row.get::<String, _>("parameters_json"))?,
                })
            })
            .collect()
    }

    async fn evidence_feed(
        &self,
        warehouse: &HashMap<String, &WarehouseEntryView>,
    ) -> Result<Vec<IntelligenceItemView>, AppError> {
        let rows = sqlx::query("SELECT id,source,source_type,asset,title,content,captured_at,reliability,confidence,raw_reference FROM evidence ORDER BY captured_at DESC LIMIT 200")
            .fetch_all(self.pool())
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let stored = warehouse.get(&id);
                IntelligenceItemView {
                    id: id.clone(),
                    source_method: source_method(&row.get::<String, _>("source_type")),
                    source_app: row.get("source"),
                    source_account: None,
                    title: row.get("title"),
                    summary: row.get("content"),
                    observed_at: row.get("captured_at"),
                    data_type: "evidence".into(),
                    evidence_status: "validated".into(),
                    evidence_quality: Some(row.get("reliability")),
                    confidence: row.get("confidence"),
                    assets: vec![row.get("asset")],
                    favorite: stored.is_some_and(|entry| entry.favorite),
                    saved: stored.is_some_and(|entry| entry.saved),
                    official_source: false,
                    has_contradiction: false,
                    mobile_observation_id: None,
                    evidence_ids: vec![id],
                    related_signal_ids: Vec::new(),
                    source_locator: row.get("raw_reference"),
                    visible_facts: Vec::new(),
                }
            })
            .collect())
    }
}

fn warehouse_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<WarehouseEntryView, AppError> {
    Ok(WarehouseEntryView {
        id: row.get("id"),
        item_id: row.get("item_id"),
        favorite: row.get("favorite"),
        saved: row.get("saved"),
        tags: serde_json::from_str(&row.get::<String, _>("tags_json"))?,
        note: row.get("note"),
        collection: row.get("collection_name"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

async fn related_signals(
    pool: &sqlx::SqlitePool,
    assets: &[String],
) -> Result<Vec<String>, AppError> {
    let mut ids = Vec::new();
    for asset in assets {
        let related: Vec<String> =
            sqlx::query_scalar("SELECT id FROM signals WHERE UPPER(asset)=UPPER(?)")
                .bind(asset)
                .fetch_all(pool)
                .await?;
        for id in related {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

fn enum_value<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".into())
        .trim_matches('"')
        .to_owned()
}

fn source_method(source_type: &str) -> String {
    match source_type.to_ascii_lowercase().as_str() {
        "websocket" => "WEBSOCKET",
        "rss" => "RSS",
        "html" => "HTML",
        "web" => "WEB",
        "desktop" => "DESKTOP",
        _ => "API",
    }
    .into()
}
