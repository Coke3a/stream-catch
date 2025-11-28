use anyhow::Result;
use diesel::{
    Connection, PgConnection,
    connection::CacheSize,
    r2d2::{ConnectionManager, CustomizeConnection, Error as R2d2Error, Pool},
};

#[derive(Debug, Default)]
struct DisablePreparedStatements;

impl CustomizeConnection<PgConnection, R2d2Error> for DisablePreparedStatements {
    fn on_acquire(&self, conn: &mut PgConnection) -> std::result::Result<(), R2d2Error> {
        conn.set_prepared_statement_cache_size(CacheSize::Disabled);
        Ok(())
    }
}

pub type PgPoolSquad = Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection(database_url: &str) -> Result<PgPoolSquad> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .connection_customizer(Box::new(DisablePreparedStatements::default()))
        .build(manager)?;
    Ok(pool)
}



// Old version using in production
// use anyhow::Result;
// use diesel::{
//     PgConnection,
//     r2d2::{ConnectionManager, Pool},
// };

// pub type PgPoolSquad = Pool<ConnectionManager<PgConnection>>;

// pub fn establish_connection(database_url: &str) -> Result<PgPoolSquad> {
//     let manager = ConnectionManager::<PgConnection>::new(database_url);
//     let pool = Pool::builder().build(manager)?;
//     Ok(pool)
// }
