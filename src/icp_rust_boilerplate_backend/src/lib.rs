#[macro_use]
#[warn(unused_imports)]
#[warn(unused_mut)]

extern crate serde;
use std::cell::RefMut;
use ic_stable_structures::Ic0StableMemory;
// use core::cell::RefMut;
use candid::{Decode, Encode, Principal};
use ic_cdk::api::{time, caller};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Task {
    owner: Principal,
    id: u64,
    title: String,
    description: String,
    completed: bool,
    created_at: u64,
    updated_at: Option<u64>,
    deadline: Option<u64>,
    completed_late: bool,
}

impl Storable for Task {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Task {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Task, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TaskPayload {
    title: String,
    description: String,
    deadline: Option<u64>,
}

#[ic_cdk::query]
fn get_task(id: u64) -> Result<Task, Error> {
    match _get_task(&id) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("Task with id={} not found", id),
        }),
    }
}

#[ic_cdk::query]
fn get_completed_tasks() -> Result<Vec<Task>, Error> {
    let tasksmap: Vec<(u64, Task)> = STORAGE.with(|service| service.borrow().iter().collect());
    let completed_tasks: Vec<Task> = tasksmap
        .into_iter()
        .filter(|(_, task)| task.completed)
        .map(|(_, task)| task)
        .collect();
    if completed_tasks.is_empty() {
        Err(Error::NotFound {
            msg: "There are currently no completed tasks".to_string(),
        })
    } else {
        Ok(completed_tasks)
    }
}

#[ic_cdk::query]
fn get_all_tasks() -> Result<Vec<Task>, Error> {
    let tasks: Vec<Task> = STORAGE
        .with(|service| service.borrow().iter().map(|(_, task)| task.clone()).collect());
    if tasks.is_empty() {
        Err(Error::NotFound {
            msg: "There are currently no tasks".to_string(),
        })
    } else {
        Ok(tasks)
    }
}

#[ic_cdk::update]
fn add_task(payload: TaskPayload) -> Option<Task> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");
    let task = Task {
        owner: caller(),
        id,
        title: payload.title,
        description: payload.description,
        completed: false,
        created_at: time(),
        updated_at: None,
        deadline: payload.deadline,
        completed_late: false,
    };
    do_insert(&task);
    Some(task)
}

#[ic_cdk::update]
fn update_task(id: u64, payload: TaskPayload) -> Result<Task, Error> {
  let updated_task = STORAGE.with(|service| {
      let mut service_ref_mut = service.borrow_mut();
      if let Some(task) = service_ref_mut.get(&id) {
          if task.owner != caller() {
              return Err(Error::NotAuthorized {
                  msg: format!("Caller is not authorized to update task with id={}", id),
                  caller: caller(),
              });
          }
          // Clone the task and update its fields
          let mut updated_task = task.clone();
          updated_task.title = payload.title.clone();
          updated_task.description = payload.description.clone();
          updated_task.deadline = payload.deadline;
          updated_task.updated_at = Some(time());
          // Replace the task in the map
          service_ref_mut.insert(id, updated_task.clone());
          Ok(updated_task)
      } else {
          Err(Error::NotFound {
              msg: format!("Task with id={} not found", id),
          })
      }
  });

  updated_task
}


#[ic_cdk::update]
fn complete_task(id: u64) -> Result<Task, Error> {
  let completed_task = STORAGE.with(|service| {
      let mut service_ref_mut = service.borrow_mut();
      if let Some(mut task) = service_ref_mut.remove(&id) {
          // Check if caller is authorized to complete the task
          if task.owner != caller() {
              return Err(Error::NotAuthorized {
                  msg: format!("Caller is not authorized to complete task with id={}", id),
                  caller: caller(),
              });
          }
          
          // Check if task is already completed
          if task.completed {
              return Err(Error::InvalidAction {
                  msg: "Task is already completed".to_string(),
              });
          }
          
          // Mark the task as completed
          task.completed = true;
          
          // Check if task is completed late
          if let Some(deadline) = task.deadline {
              if time() > deadline {
                  task.completed_late = true;
              }
          }
          
          // Update the task's updated_at field
          task.updated_at = Some(time());
          
          // Clone the task before re-inserting it
          let cloned_task = task.clone();
          
          // Re-insert the modified task
          service_ref_mut.insert(id, task);
          
          Ok(cloned_task)
      } else {
          Err(Error::NotFound {
              msg: format!("Task with id={} not found", id),
          })
      }
  });

  completed_task
}


#[ic_cdk::update]
fn delete_task(id: u64) -> Result<Task, Error> {
    let task = _get_task(&id).ok_or(Error::NotFound {
        msg: format!("Task with id={} not found", id),
    })?;
    if task.owner != caller() {
        return Err(Error::NotAuthorized {
            msg: format!("Caller is not authorized to delete task with id={}", id),
            caller: caller(),
        });
    }
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("Task with id={} not found", id),
        }),
    }
}

fn do_insert(task: &Task) {
    STORAGE.with(|service| service.borrow_mut().insert(task.id, task.clone()));
}

fn _get_task(id: &u64) -> Option<Task> {
    STORAGE.with(|service| service.borrow().get(id).map(|task_ref| task_ref.clone()))
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    NotAuthorized { msg: String, caller: Principal },
    InvalidAction { msg: String },
}

// Need this to generate candid
ic_cdk::export_candid!();
