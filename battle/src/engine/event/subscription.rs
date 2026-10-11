use crate::engine::{
    event::kind::EventKind,
    skill::{action::SkillPhase, rule::DefinitionKey},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PublicationPhase {
    BeforePublish,
    #[default]
    AfterPublish,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ReactionTiming {
    #[default]
    Immediate,
    AfterSkill,
    AfterHitSkills,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionKey {
    pub event: EventKind,
    pub definition: DefinitionKey,
    pub phase: Option<SkillPhase>,
    pub publication: PublicationPhase,
    pub timing: ReactionTiming,
}

impl SubscriptionKey {
    pub const fn new(event: EventKind, definition: DefinitionKey) -> Self {
        Self {
            event,
            definition,
            phase: None,
            publication: PublicationPhase::AfterPublish,
            timing: ReactionTiming::Immediate,
        }
    }

    pub const fn at_phase(
        event: EventKind,
        definition: DefinitionKey,
        phase: Option<SkillPhase>,
    ) -> Self {
        Self {
            event,
            definition,
            phase,
            publication: PublicationPhase::AfterPublish,
            timing: ReactionTiming::Immediate,
        }
    }

    pub const fn with_publication(mut self, publication: PublicationPhase) -> Self {
        self.publication = publication;
        self
    }

    pub const fn with_timing(mut self, timing: ReactionTiming) -> Self {
        self.timing = timing;
        self
    }

    pub const fn at_phase_and_publication(
        event: EventKind,
        definition: DefinitionKey,
        phase: Option<SkillPhase>,
        publication: PublicationPhase,
    ) -> Self {
        Self {
            event,
            definition,
            phase,
            publication,
            timing: ReactionTiming::Immediate,
        }
    }
}
