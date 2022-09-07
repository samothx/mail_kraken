use crate::{Config, UserId};
use mysql_async::Pool;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct SharedData {
    pub config: Config,
    pub db_conn: Option<Pool>,
    // pub task_list: Vec<JoinHandle<Result<()>>>,
    pub user_id: Option<UserId>,
}

#[derive(Clone)]
pub struct StateData {
    data: Arc<RwLock<SharedData>>,
}

impl StateData {
    pub fn new(data: SharedData) -> StateData {
        StateData {
            data: Arc::new(RwLock::new(data)),
        }
    }

    pub fn get_state(&self) -> LockResult<RwLockReadGuard<SharedData>> {
        self.data.read()
    }

    pub fn get_mut_state(&self) -> LockResult<RwLockWriteGuard<SharedData>> {
        self.data.write()
    }

    /*pub fn get_db_conn(&self) -> Result<Option<Pool>> {
        Ok(self.data.read()?.db_conn.clone())
    }
     */
}
