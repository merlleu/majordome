use majordome::MajordomeError;
use scylla::FromRow;

pub trait MjScyllaORM {
    type Updater;
    fn update(self) -> Self::Updater;
    // fn select<T>(&self) -> T;
    fn table_name() -> &'static str;
    fn query() -> &'static str;
}

pub struct MajordomeScyllaSelectResult<T: FromRow + MjScyllaORM> {
    pub resp: smallvec::SmallVec<[T; 1]>,
}

impl<T> MajordomeScyllaSelectResult<T>
where
    T: FromRow + MjScyllaORM,
{
    pub fn one(self) -> Result<T, MajordomeError> {
        match self.resp.len() {
            0 => crate::err::ScyllaORMError::NotFoundExpectedOne(T::table_name().to_string()).err(),
            1 => Ok(self.resp.into_iter().next().unwrap()),
            count => crate::err::ScyllaORMError::TooManyResultsExpectedOne(
                T::table_name().to_string(),
                count,
            )
            .err(),
        }
    }

    pub fn all(self) -> smallvec::SmallVec<[T; 1]> {
        self.resp
    }

    pub fn len(&self) -> usize {
        self.resp.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resp.is_empty()
    }
}
