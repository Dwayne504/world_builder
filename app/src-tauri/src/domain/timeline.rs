//! Fictional chronology. Dates are calendar coordinates, never wall-clock timestamps.
use super::{
    story::WorkspaceState,
    structure::{ChapterId, OccurrenceId},
    EntryId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalendarMonth {
    pub name: String,
    pub days: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Calendar {
    pub name: String,
    pub era_label: String,
    pub months: Vec<CalendarMonth>,
}
impl Calendar {
    pub fn validate(&self) -> Result<(), String> {
        super::require_definition_name(&self.name).map_err(|e| e.to_string())?;
        if self.era_label.chars().count() > 80 {
            return Err("Era labels may contain up to 80 characters".into());
        }
        if self.months.is_empty() || self.months.len() > 60 {
            return Err("Use between 1 and 60 months".into());
        }
        let mut names = std::collections::HashSet::new();
        for month in &self.months {
            let name = super::require_definition_name(&month.name).map_err(|e| e.to_string())?;
            if !names.insert(name.to_lowercase()) {
                return Err("Give each month a different name".into());
            }
            if !(1..=1000).contains(&month.days) {
                return Err("Each month needs between 1 and 1,000 days".into());
            }
        }
        Ok(())
    }
    pub fn same_structure(&self, other: &Self) -> bool {
        self.months
            .iter()
            .map(|m| m.days)
            .eq(other.months.iter().map(|m| m.days))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FictionalDate {
    pub year: i32,
    pub month: u16,
    pub day: u16,
}
impl FictionalDate {
    pub fn validate(&self, calendar: &Calendar) -> Result<(), String> {
        if !(-1_000_000..=1_000_000).contains(&self.year) {
            return Err("Use a year from −1,000,000 to 1,000,000".into());
        }
        let month = self
            .month
            .checked_sub(1)
            .and_then(|m| calendar.months.get(usize::from(m)))
            .ok_or("Choose a month in this calendar")?;
        if self.day == 0 || self.day > month.days {
            return Err(format!("{} has days 1–{}", month.name, month.days));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineLink {
    pub id: String,
    pub label: String,
    pub workspace_state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub id: OccurrenceId,
    pub title: String,
    pub notes: String,
    pub date: Option<FictionalDate>,
    pub event_entry: Option<TimelineLink>,
    pub entries: Vec<TimelineLink>,
    pub chapters: Vec<TimelineLink>,
    pub workspace_state: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSnapshot {
    pub global_revision: i64,
    pub calendar: Option<Calendar>,
    pub occurrences: Vec<Occurrence>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OccurrenceDraft {
    pub title: String,
    pub notes: String,
    pub date: Option<FictionalDate>,
    pub event_entry_id: Option<EntryId>,
    pub entry_ids: Vec<EntryId>,
    pub chapter_ids: Vec<ChapterId>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TimelineCommand {
    ConfigureCalendar {
        calendar: Calendar,
    },
    Create,
    Save {
        id: OccurrenceId,
        draft: OccurrenceDraft,
    },
    SetState {
        id: OccurrenceId,
        state: WorkspaceState,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fictional_dates_use_configured_months_and_include_year_zero() {
        let calendar = Calendar {
            name: "Orbit".into(),
            era_label: "After landing".into(),
            months: vec![
                CalendarMonth {
                    name: "Dawn".into(),
                    days: 40,
                },
                CalendarMonth {
                    name: "Dusk".into(),
                    days: 8,
                },
            ],
        };
        calendar.validate().unwrap();
        for year in [-1_000_000, -1, 0, 1, 1_000_000] {
            FictionalDate {
                year,
                month: 1,
                day: 40,
            }
            .validate(&calendar)
            .unwrap();
        }
        for date in [
            FictionalDate {
                year: 0,
                month: 0,
                day: 1,
            },
            FictionalDate {
                year: 0,
                month: 2,
                day: 9,
            },
            FictionalDate {
                year: 0,
                month: 1,
                day: 0,
            },
            FictionalDate {
                year: 1_000_001,
                month: 1,
                day: 1,
            },
        ] {
            assert!(date.validate(&calendar).is_err());
        }
        let mut invalid = calendar.clone();
        invalid.months[1].name = "dawn".into();
        assert!(invalid.validate().is_err());
        invalid.months.clear();
        assert!(invalid.validate().is_err());
    }
}
