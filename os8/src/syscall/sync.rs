use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use crate::config::{DEADLOCK_DETECT_RESOURCE_KIND_MUTEX, DEADLOCK_DETECT_RESOURCE_KIND_SEMAPHORE};

pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

// LAB5 HINT: you might need to maintain data structures used for deadlock detection
// during sys_mutex_* and sys_semaphore_* syscalls
pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    };

    process_inner.deadlock_detect_work.insert((DEADLOCK_DETECT_RESOURCE_KIND_MUTEX as u32,id as u32), 1);

    id
}

// LAB5 HINT: Return -0xDEAD if deadlock is detected
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();

    task_inner.deadlock_detect.deadlock_detect_block_because_need = Some((DEADLOCK_DETECT_RESOURCE_KIND_MUTEX as u32, mutex_id as u32, 1));


    drop(task_inner);
    drop(cur_task);


    if process_inner.deadlock_detect_enabled {
        if process_inner.detect_will_deadlock() {
            let cur_task = current_task().unwrap();
            let mut task_inner = cur_task.inner_exclusive_access();
            task_inner.deadlock_detect.deadlock_detect_block_because_need = None;
            return -0xDEAD
        }
    }


    drop(process_inner);
    drop(process);
    mutex.lock();

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();
    let d = &mut task_inner.deadlock_detect;
    d.deadlock_detect_block_because_need = None;

    let key = (DEADLOCK_DETECT_RESOURCE_KIND_MUTEX as u32, mutex_id as u32);
    if let Some(v) = d.deadlock_detect_allocation.get_mut(&key) {
        *v += 1;
    } else {
        d.deadlock_detect_allocation.insert(key, 1);
    }
    



    0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();
    let d = &mut task_inner.deadlock_detect;

    let key = (DEADLOCK_DETECT_RESOURCE_KIND_MUTEX as u32, mutex_id as u32);
    if let Some(v) = d.deadlock_detect_allocation.get_mut(&key) {
        *v -= 1;
    }
    drop(task_inner);
    drop(cur_task);

    mutex.unlock();
    0
}

pub fn sys_semaphore_create(res_count: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    process_inner.deadlock_detect_work.insert((DEADLOCK_DETECT_RESOURCE_KIND_SEMAPHORE as u32, id as u32), res_count as u32);
    id as isize
}

pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);


    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();

    println!("tid={} sema = {} sema up begin", task_inner.res.as_ref().unwrap().tid, sem_id);

    let d = &mut task_inner.deadlock_detect;

    let key = (DEADLOCK_DETECT_RESOURCE_KIND_SEMAPHORE as u32, sem_id as u32);
    if let Some(v) = d.deadlock_detect_allocation.get_mut(&key) {
        *v -= 1;
    }
    drop(task_inner);
    drop(cur_task);


    sem.up();


    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();
    println!("tid={} sema = {} sema up end", task_inner.res.as_ref().unwrap().tid, sem_id);

    0
}

// LAB5 HINT: Return -0xDEAD if deadlock is detected
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();


    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();

    println!("tid={} sema={} sema down begin", task_inner.res.as_ref().unwrap().tid, sem_id);

    if sem_id == 2 {
        println!("xxxx");
    }
    task_inner.deadlock_detect.deadlock_detect_block_because_need = Some((DEADLOCK_DETECT_RESOURCE_KIND_SEMAPHORE as u32, sem_id as u32, 1));


    drop(task_inner);
    drop(cur_task);


    if process_inner.deadlock_detect_enabled {
        if process_inner.detect_will_deadlock() {
            let cur_task = current_task().unwrap();
            let mut task_inner = cur_task.inner_exclusive_access();
            task_inner.deadlock_detect.deadlock_detect_block_because_need = None;
            println!("sema dead tid = {}, sem={}", task_inner.res.as_ref().unwrap().tid, sem_id);
            return -0xDEAD
        }
    }


    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    
    sem.down();

    let cur_task = current_task().unwrap();
    let mut task_inner = cur_task.inner_exclusive_access();
    let d = &mut task_inner.deadlock_detect;
    d.deadlock_detect_block_because_need = None;

    let key = (DEADLOCK_DETECT_RESOURCE_KIND_SEMAPHORE as u32, sem_id as u32);
    if let Some(v) = d.deadlock_detect_allocation.get_mut(&key) {
        *v += 1;
    } else {
        d.deadlock_detect_allocation.insert(key, 1);
    }

    println!("tid={} sema = {} sema down end", task_inner.res.as_ref().unwrap().tid, sem_id);

    0
}

pub fn sys_condvar_create(_arg: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}

pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}

pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}

// LAB5 YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    match _enabled {
        0 => process_inner.deadlock_detect_enabled = false,
        1 => process_inner.deadlock_detect_enabled = true,
        _ => return -1,
    }
    0
}
