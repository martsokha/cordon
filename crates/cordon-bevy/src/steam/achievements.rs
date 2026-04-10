//! Achievement definitions.

use strum::IntoStaticStr;

/// All achievements in the game. The string representation must
/// match what's configured in the Steamworks partner portal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoStaticStr)]
pub enum Achievement {
    #[strum(serialize = "ACH_FIRST_KILL")]
    FirstKill,
    #[strum(serialize = "ACH_SQUAD_WIPE")]
    SquadWipe,
    #[strum(serialize = "ACH_OPEN_FOR_BUSINESS")]
    OpenForBusiness,
    #[strum(serialize = "ACH_RICH")]
    Rich,
    #[strum(serialize = "ACH_EXPLORE_ALL")]
    ExploreAll,
    #[strum(serialize = "ACH_CCTV_PEEK")]
    CctvPeek,
    #[strum(serialize = "ACH_FIRST_RELIC")]
    FirstRelic,
    #[strum(serialize = "ACH_SURVIVE_7")]
    Survive7,
    #[strum(serialize = "ACH_FIRST_QUEST")]
    FirstQuest,
}

impl Achievement {
    pub fn api_name(self) -> &'static str {
        self.into()
    }
}
