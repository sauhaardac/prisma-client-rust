use dml::PrismaValue;
use query_core::{Operation, Selection, SelectionArgument};
use serde::{de::DeserializeOwned, Serialize};

use crate::{PrismaClientInternals, WhereInput};

pub trait QueryConvert {
    type RawType: Data;
    type ReturnValue: Serialize + 'static;

    /// Function for converting between raw database data and the type expected by the user.
    /// Necessary for things like raw queries
    fn convert(raw: Self::RawType) -> Self::ReturnValue;
}

pub trait Query<'a>: QueryConvert {
    fn graphql(self) -> (Operation, &'a PrismaClientInternals);
}

pub trait ModelActions {
    type Data: Data;
    type Where: WhereInput;
    type Set: Into<(String, PrismaValue)>;
    type With: Into<Selection>;
    type OrderBy: Into<(String, PrismaValue)>;
    type Cursor: Into<Self::Where>;

    const MODEL: &'static str;

    fn scalar_selections() -> Vec<Selection>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelReadOperation {
    FindUnique,
    FindFirst,
    FindMany,
    Count,
}

impl ModelReadOperation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FindUnique => "findUnique",
            Self::FindFirst => "findFirst",
            Self::FindMany => "findMany",
            Self::Count => "aggregate",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelWriteOperation {
    Create,
    CreateMany,
    Update,
    UpdateMany,
    Delete,
    DeleteMany,
    Upsert,
}

impl ModelWriteOperation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Create => "createOne",
            Self::CreateMany => "createMany",
            Self::Update => "updateOne",
            Self::UpdateMany => "updateMany",
            Self::Delete => "deleteOne",
            Self::DeleteMany => "deleteMany",
            Self::Upsert => "upsertOne",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelOperation {
    Read(ModelReadOperation),
    Write(ModelWriteOperation),
}

impl ModelOperation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Read(q) => q.name(),
            Self::Write(q) => q.name(),
        }
    }
}

pub trait ModelQuery<'a>: Query<'a> {
    type Actions: ModelActions;

    const TYPE: ModelOperation;

    fn base_selection(
        arguments: impl IntoIterator<Item = SelectionArgument>,
        nested_selections: impl IntoIterator<Item = Selection>,
    ) -> Selection {
        Selection::new(
            format!("{}{}", Self::TYPE.name(), Self::Actions::MODEL),
            Some("result".to_string()),
            arguments.into_iter().collect::<Vec<_>>(),
            nested_selections.into_iter().collect::<Vec<_>>(),
        )
    }
}

pub trait WhereQuery<'a>: ModelQuery<'a> {
    fn add_where(&mut self, param: <<Self as ModelQuery<'a>>::Actions as ModelActions>::Where);
}

pub trait WithQuery<'a>: ModelQuery<'a> {
    fn add_with(
        &mut self,
        param: impl Into<<<Self as ModelQuery<'a>>::Actions as ModelActions>::With>,
    );
}

pub trait OrderByQuery<'a>: ModelQuery<'a> {
    fn add_order_by(&mut self, param: <<Self as ModelQuery<'a>>::Actions as ModelActions>::OrderBy);
}

pub trait PaginatedQuery<'a>: ModelQuery<'a> {
    fn add_cursor(&mut self, param: <<Self as ModelQuery<'a>>::Actions as ModelActions>::Cursor);
    fn set_skip(&mut self, skip: i64);
    fn set_take(&mut self, take: i64);
}

pub trait SetQuery<'a>: ModelQuery<'a> {
    fn add_set(&mut self, param: <<Self as ModelQuery<'a>>::Actions as ModelActions>::Set);
}

pub trait Data: DeserializeOwned + 'static {}

impl<T: DeserializeOwned + 'static> Data for T {}
