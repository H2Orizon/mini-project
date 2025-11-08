use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PasswordResets::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PasswordResets::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(PasswordResets::UserId).integer().not_null())
                    .col(ColumnDef::new(PasswordResets::TokenHash).string().not_null())
                    .col(ColumnDef::new(PasswordResets::ExpiresAt).timestamp().not_null())
                    .col(ColumnDef::new(PasswordResets::Used).boolean().not_null().default(false))
                    .foreign_key(
                        ForeignKey::create()
                            .from(PasswordResets::Table, PasswordResets::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PasswordResets::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum PasswordResets {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    Used,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
