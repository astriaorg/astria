use astria_core::upgrades::v1::{
    ChangeName,
    UpgradeName,
};

pub(in crate::upgrades) fn change(upgrade_name: &UpgradeName, change_name: &ChangeName) -> String {
    format!("upgrades/{upgrade_name}/{change_name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "upgrades/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(change(
            &UpgradeName::new("aspen"),
            &ChangeName::new("change_1")
        ));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(
            change(&UpgradeName::new("aspen"), &ChangeName::new("change_1"))
                .starts_with(COMPONENT_PREFIX)
        );
    }
}
