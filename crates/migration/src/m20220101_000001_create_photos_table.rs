use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Photo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Photo::Id)
                            .integer()
                            .primary_key()
                            .not_null()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(Photo::Hash).string().unique_key().not_null())
                    .col(ColumnDef::new(Photo::Title).string())
                    .col(ColumnDef::new(Photo::Artist).string())
                    .col(ColumnDef::new(Photo::Copyright).string())
                    .col(ColumnDef::new(Photo::Notes).string())
                    .col(ColumnDef::new(Photo::DateTaken).date_time())
                    .col(ColumnDef::new(Photo::FullsizePath).string().not_null())
                    .col(ColumnDef::new(Photo::WebsizePath).string().not_null())
                    .col(ColumnDef::new(Photo::ThumbnailPath).string().not_null())
                    .col(
                        ColumnDef::new(Photo::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Photo::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Photo::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Photo {
    Table,
    Id,
    Hash,
    Title,
    Notes,
    Artist,
    Copyright,
    DateTaken,
    FullsizePath,
    WebsizePath,
    ThumbnailPath,
    CreatedAt,
    UpdatedAt,
}
