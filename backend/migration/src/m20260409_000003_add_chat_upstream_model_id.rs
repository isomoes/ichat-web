use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Chat::Table)
                    .add_column_if_not_exists(string_null(Chat::UpstreamModelId))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Chat::Table)
                    .drop_column(Chat::UpstreamModelId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Chat {
    Table,
    UpstreamModelId,
}
