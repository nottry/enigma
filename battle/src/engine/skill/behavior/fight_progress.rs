use crate::engine::{
    manager::fight_progress::FightProgressCommand,
    skill::{
        behavior::{BehaviorOpContext, registry::BehaviorHandler},
        effect::ParsedBehavior,
        rule::output::{BattleCommand, RuleOp},
    },
};

pub(super) struct InitializeHandler;

impl BehaviorHandler for InitializeHandler {
    const VALIDATES_ARGUMENTS: bool = true;

    fn supports(behavior: &ParsedBehavior) -> bool {
        matches!(
            behavior.args.as_slice(),
            [initial, max, show_id, progress_id]
                if *max > 0
                    && (0..=*max).contains(initial)
                    && *show_id > 0
                    && *progress_id > 0
        )
    }

    fn emit_ops(context: BehaviorOpContext<'_>, behavior: &ParsedBehavior) -> Option<Vec<RuleOp>> {
        Self::supports(behavior).then(|| {
            vec![RuleOp::Command(BattleCommand::FightProgress(
                FightProgressCommand {
                    origin: super::command_origin(behavior).expect("registered behavior"),
                    initial: behavior.args[0],
                    max: behavior.args[1],
                    show_id: behavior.args[2],
                    progress_id: behavior.args[3],
                    skill_id: context.active_skill_id,
                },
            ))]
        })
    }
}
