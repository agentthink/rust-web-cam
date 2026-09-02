use axum::{extract::State, Json};
use rbatis::RBatisRef;
use std::sync::Arc;

use crate::api::response::ApiResponse;
use crate::api::state::FullState;
use crate::domain::gb28181_ref::{GbDeviceType, GbIndustryCode, GbNetworkCode, GbReferenceData};
use crate::error::Result;
use crate::sql_mappers::{gb_device_type_select_all, gb_industry_code_select_all, gb_network_code_select_all};

pub async fn get_gb28181_ref_data_handler(
    State(state): State<Arc<FullState>>,
) -> Result<Json<ApiResponse<GbReferenceData>>> {
    let conn = state.app.registry.infra.db.rb().acquire().await?;

    let device_types: Vec<GbDeviceType> = gb_device_type_select_all(conn.rb_ref())
        .await?
        .into_iter()
        .map(GbDeviceType::from)
        .collect();

    let industry_codes: Vec<GbIndustryCode> = gb_industry_code_select_all(conn.rb_ref())
        .await?
        .into_iter()
        .map(GbIndustryCode::from)
        .collect();

    let network_codes: Vec<GbNetworkCode> = gb_network_code_select_all(conn.rb_ref())
        .await?
        .into_iter()
        .map(GbNetworkCode::from)
        .collect();

    Ok(Json(ApiResponse::success(GbReferenceData {
        device_types,
        industry_codes,
        network_codes,
    })))
}
