use sonettobuf::{Fight, FightProgressInfo};

use crate::engine::skill::rule::CommandOrigin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightProgressState {
    pub id: i32,
    pub max: i32,
    pub value: i32,
    pub show_id: i32,
    pub skill_id: i32,
}

impl From<FightProgressState> for FightProgressInfo {
    fn from(state: FightProgressState) -> Self {
        Self {
            id: Some(state.id),
            max: Some(state.max),
            value: Some(state.value),
            show_id: Some(state.show_id),
            skill_id: Some(state.skill_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightProgressCommand {
    pub origin: CommandOrigin,
    pub initial: i32,
    pub max: i32,
    pub show_id: i32,
    pub progress_id: i32,
    pub skill_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightProgressChange {
    pub origin: CommandOrigin,
    pub state: FightProgressState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightProgressError {
    InvalidDefinition,
    AlreadyInitialized(i32),
}

#[derive(Debug, Clone, Default)]
pub struct FightProgressManager {
    records: Vec<FightProgressInfo>,
}

impl FightProgressManager {
    pub fn seed(&mut self, fight: &Fight) {
        self.records.clone_from(&fight.progress_list);
    }

    pub fn initialize(
        &mut self,
        command: FightProgressCommand,
    ) -> Result<FightProgressChange, FightProgressError> {
        if command.progress_id <= 0
            || command.max <= 0
            || !(0..=command.max).contains(&command.initial)
            || command.show_id <= 0
            || command.skill_id <= 0
        {
            return Err(FightProgressError::InvalidDefinition);
        }
        if self
            .records
            .iter()
            .any(|record| record.id == Some(command.progress_id))
        {
            return Err(FightProgressError::AlreadyInitialized(command.progress_id));
        }
        let state = FightProgressState {
            id: command.progress_id,
            max: command.max,
            value: command.initial,
            show_id: command.show_id,
            skill_id: command.skill_id,
        };
        self.records.push(state.into());
        Ok(FightProgressChange {
            origin: command.origin,
            state,
        })
    }

    pub fn get(&self, progress_id: i32) -> Option<FightProgressState> {
        let record = self
            .records
            .iter()
            .find(|record| record.id == Some(progress_id))?;
        Some(FightProgressState {
            id: record.id?,
            max: record.max?,
            value: record.value?,
            show_id: record.show_id?,
            skill_id: record.skill_id?,
        })
    }

    pub fn sync_fight(&self, fight: &mut Fight) {
        fight.progress_list.clone_from(&self.records);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::skill::rule::{DefinitionKey, RuleDomain};

    #[test]
    fn initialization_owns_the_complete_progress_record() {
        let origin = CommandOrigin {
            domain: RuleDomain::Behavior,
            key: DefinitionKey::new(100015, "ProgressInitNew"),
        };
        let mut manager = FightProgressManager::default();
        let change = manager
            .initialize(FightProgressCommand {
                origin,
                initial: 0,
                max: 100,
                show_id: 10,
                progress_id: 8,
                skill_id: 119381117,
            })
            .unwrap();

        assert_eq!(change.origin, origin);
        assert_eq!(change.state, manager.get(8).unwrap());
        assert_eq!(change.state.value, 0);
        assert_eq!(change.state.max, 100);
        assert!(matches!(
            manager.initialize(FightProgressCommand {
                origin,
                initial: 0,
                max: 100,
                show_id: 10,
                progress_id: 8,
                skill_id: 119381117,
            }),
            Err(FightProgressError::AlreadyInitialized(8))
        ));
    }

    #[test]
    fn snapshot_round_trip_preserves_incomplete_and_duplicate_records() {
        let records = vec![
            FightProgressInfo {
                id: Some(8),
                max: Some(100),
                ..Default::default()
            },
            FightProgressInfo {
                id: Some(8),
                value: Some(50),
                ..Default::default()
            },
            FightProgressInfo::default(),
        ];
        let mut manager = FightProgressManager::default();
        manager.seed(&Fight {
            progress_list: records.clone(),
            ..Default::default()
        });
        let mut fight = Fight::default();

        manager.sync_fight(&mut fight);

        assert_eq!(fight.progress_list, records);
    }
}
