use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .table(LoginAttempts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LoginAttempts::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(LoginAttempts::UserEmail).string().not_null())
                    .col(ColumnDef::new(LoginAttempts::IpAddress).string().null())
                    .col(ColumnDef::new(LoginAttempts::Provider).string().null())
                    .col(ColumnDef::new(LoginAttempts::Success).boolean().not_null())
                    .col(ColumnDef::new(LoginAttempts::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(LoginAttempts::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum LoginAttempts {
    Table,
    Id,
    UserEmail,
    IpAddress,
    Provider,
    Success,
    CreatedAt,
}
