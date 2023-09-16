use sea_orm::{ColumnTrait, PrimaryKeyTrait, QueryOrder, QuerySelect};
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter, Select};
use tonic::{async_trait, Request, Response};
// use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::common::error::result_into;
use crate::endpoint::{Auth, ControllerTrait};
use crate::grpc::proto::prelude::{ListRequest, Page, SearchByTextRequest, SortBy};
use crate::init::db::DB;

#[async_trait]
pub trait Endpoint<I>
where
    I: IntellisenseTrait,
    Self: Intellisense<I> + ControllerTrait,
    SortBy: Into<<<I as IntellisenseTrait>::Entity as EntityTrait>::Column>,
    <I as IntellisenseTrait>::InfoArray: From<Vec<<I as IntellisenseTrait>::Info>>,
    <I as IntellisenseTrait>::Info: From<<<I as IntellisenseTrait>::Entity as EntityTrait>::Model>,
    <I as IntellisenseTrait>::PrimaryKey: Into<<<<I as IntellisenseTrait>::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType>+Send,
{
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<<I as IntellisenseTrait>::InfoArray>, tonic::Status> {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let mut query = I::Entity::find();
        Self::ro_filter(&mut query, auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
            .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = request.reverse;

        Self::sort_filter(&mut query, sort_by, page, reverse);

        let list: Vec<<I as IntellisenseTrait>::Info> = result_into(query.all(db).await)?
            .into_iter()
            .map(|x| x.into())
            .collect();
        Ok(Response::new(list.into()))
    }
    async fn search_by_text(
        &self,
        request: Request<SearchByTextRequest>,
        text: &'static [<<I as IntellisenseTrait>::Entity as EntityTrait>::Column],
    ) -> Result<Response<<I as IntellisenseTrait>::InfoArray>, tonic::Status> {
        debug_assert!(text.len() > 0);
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let mut query = I::Entity::find();
        Self::ro_filter(&mut query, auth)?;

        let sort_by = SortBy::from_i32(request.sort_by)
            .ok_or(tonic::Status::invalid_argument("Invalid sort_by"))?;
        let page = request
            .page
            .ok_or(tonic::Status::invalid_argument("Invalid page"))?;
        let reverse = false;

        Self::sort_filter(&mut query, sort_by, page, reverse);

        let mut condition = text[0].like(request.text.clone());
        for col in text[1..].iter() {
            condition = condition.or(col.like(request.text.clone()));
        }

        query = query.filter(condition);

        let list: Vec<<I as IntellisenseTrait>::Info> = result_into(query.all(db).await)?
            .into_iter()
            .map(|x| x.into())
            .collect();
        Ok(Response::new(list.into()))
    }
    async fn full_info(
        &self,
        request: Request<<I as IntellisenseTrait>::PrimaryKey>,
    ) -> Result<Response<<I as IntellisenseTrait>::FullInfo>, tonic::Status> where 
    <I as IntellisenseTrait>::FullInfo: From<<<I as IntellisenseTrait>::Entity as EntityTrait>::Model>,
    {
        let db = DB.get().unwrap();

        let (auth, request) = self.parse_request(request).await?;

        let mut query = I::Entity::find_by_id(request.into());
        Self::ro_filter(&mut query, auth)?;

        let info: <I as IntellisenseTrait>::FullInfo = result_into(query.one(db).await)?
            .ok_or(tonic::Status::not_found("Not found"))?
            .into();
        Ok(Response::new(info))
    }
}

pub trait IntellisenseTrait
where
    Self: EntityTrait,
{
    type Entity: EntityTrait;

    type InfoArray;
    type FullInfo;
    type Info;
    type PrimaryKey;
}

pub trait Intellisense<T>
where
    T: IntellisenseTrait,
{
    fn ro_filter(self_: &mut Select<T::Entity>, auth: Auth) -> Result<(), tonic::Status>;
    fn rw_filter(self_: &mut Select<T::Entity>, auth: Auth) -> Result<(), tonic::Status>;
    fn sort_filter(self_: &mut Select<T::Entity>, sort_by: SortBy, page: Page, reverse: bool)
    where
        SortBy: Into<<<T as IntellisenseTrait>::Entity as EntityTrait>::Column>,
    {
        let col = sort_by.into();

        *self_ = if reverse {
            self_.clone().order_by_asc(col)
        } else {
            self_.clone().order_by_desc(col)
        };

        let offset = page.offset as u64;
        let limit = page.amount as u64;
        *self_ = self_.clone().offset(offset).limit(limit);
    }
}
