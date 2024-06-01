#![feature(test)]

extern crate test;

#[cfg(test)]
mod tests {
    use role_manager::analyzer::full_analysis;
    use test::Bencher;
    use chrono::Duration;
    use sea_orm::{Database, DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter};
    use serenity::model::guild::Member;
    use role_manager::config::load_config;
    use role_manager::boards::cm::CmBoardsState;
    use role_manager::boards::srcom::SrComBoardsState;
    use role_manager::analyzer::role_definition::RoleDefinition;
    use role_manager::model::lumadb::verified_connections;

    #[bench]
    fn analysis_bench(b: &mut Bencher) {
        println!("Setting up runtime");

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _guard = runtime.enter();

        // Definition file to use
        println!("Reading definition file");
        let definition: RoleDefinition = json5::from_str(include_str!("../new-propsals-23-02-25.json5")).unwrap();

        // Setup state for fetching info
        println!("Setting up state for run");
        let config = load_config();
        let db: DatabaseConnection = runtime.block_on(Database::connect(&config.database_url)).expect(
            format!("Failed to open connection to database at {}", &config.database_url).as_str()
        );
        let srcom_state = SrComBoardsState::new(Duration::minutes(15));
        let cm_state = CmBoardsState::new(Duration::minutes(15));

        let discord_http = serenity::http::Http::new(config.discord_bot_token.as_str());

        // Fetch database connections
        println!("Fetching database connections");
        let connections: Vec<verified_connections::Model> = runtime.block_on(verified_connections::Entity::find()
            .filter(verified_connections::Column::Removed.eq(0))
            .all(&db)).unwrap();

        // Fetch discord users
        println!("Fetching discord users");
        let mut users: Vec<Member> = Vec::new();
        let mut offset: Option<u64> = None;

        loop {
            let iteration = runtime.block_on(discord_http.get_guild_members(146404426746167296, Some(1_000), offset)).unwrap();
            if !iteration.is_empty() {
                offset = Some(iteration.get(iteration.len() - 1).unwrap().user.id.0);
            }

            let end = iteration.len() < 1000;

            users.extend(iteration);

            if end {
                break;
            }
        }

        // Warm-up run
        println!("Warmup run");
        let _ = runtime.block_on(full_analysis(definition.clone(), connections.clone(), users.clone(), srcom_state.clone(), cm_state.clone())).unwrap();

        println!("Go!!");
        b.iter(|| {
            runtime.block_on(full_analysis(definition.clone(), connections.clone(), users.clone(), srcom_state.clone(), cm_state.clone())).unwrap();
        })
    }
}
