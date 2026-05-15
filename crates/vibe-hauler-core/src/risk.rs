use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RiskLevel {
    Green,
    Yellow,
    Red,
    Black,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Recommendation {
    Clean,
    Review,
    Protect,
    ReportOnly,
}
