use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePlan {
    pub entities: Vec<String>,
    pub joins: Vec<String>,
    pub notes: Vec<String>,
}

/// Stub: pick minimal tables + safe join plan.
pub fn plan_tables() -> TablePlan {
    TablePlan {
        entities: vec![
            "offers_latest".into(),
            "offer_phases".into(),
            "offer_products".into(),
            "campaign_offers".into(),
            "campaigns_latest".into(),
            "partners".into(),
        ],
        joins: vec![
            "offers_latest o -> offer_phases op on (op.offer_id=o.id AND op.profile=o.profile AND op.version=o.version)".into(),
            "offers_latest o -> offer_products opr on (opr.offer_id=o.id AND opr.profile=o.profile AND opr.version=o.version)".into(),
            "offers_latest o -> campaign_offers co on (co.offer_id=o.id AND co.profile=o.profile)".into(),
            "campaign_offers co -> campaigns_latest c on (c.id=co.campaign_id AND c.profile=co.profile AND co.version=c.version)".into(),
            "campaigns_latest c -> partners p on (p.id=c.partner_id AND p.profile=c.profile)".into(),
        ],
        notes: vec!["campaign_offers.version tracks CAMPAIGN version; do not match to offer version".into()],
    }
}
