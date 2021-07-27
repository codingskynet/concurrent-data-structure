use cds::map::ConcurrentMap;
use cds::map::SequentialMap;
use cds::util::random::Random;
use crossbeam_epoch::pin;
use crossbeam_utils::thread;
use rand::prelude::SliceRandom;
use rand::prelude::ThreadRng;
use rand::thread_rng;
use rand::Rng;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
enum Operation {
    Insert,
    Lookup,
    Remove,
}

#[derive(PartialEq)]
enum OperationType {
    Some, // the operation for existing key on the map
    None, // the operation for not existing key on the map
}

pub fn stress_sequential<K, M>(iter: u64)
where
    K: Ord + Clone + Random + Debug,
    M: SequentialMap<K, u64>,
{
    // 10 times try to get not existing key, or return if failing
    let gen_not_existing_key = |rng: &mut ThreadRng, map: &BTreeMap<K, u64>| {
        let mut key = K::gen(rng);

        for _ in 0..10 {
            if !map.contains_key(&key) {
                return Ok(key);
            }

            key = K::gen(rng);
        }

        Err(())
    };

    let ops = [Operation::Insert, Operation::Lookup, Operation::Remove];
    let types = [OperationType::Some, OperationType::None];

    let mut map = M::new();
    let mut ref_map: BTreeMap<K, u64> = BTreeMap::new();
    let mut rng = thread_rng();

    for i in 1..=iter {
        let t = types.choose(&mut rng).unwrap();
        let ref_map_keys = ref_map.keys().collect::<Vec<&K>>();
        let existing_key = ref_map_keys.choose(&mut rng);

        if existing_key.is_none() || *t == OperationType::None {
            // run operation with not existing key
            let not_existing_key = if let Ok(key) = gen_not_existing_key(&mut rng, &ref_map) {
                key
            } else {
                continue;
            };

            match ops.choose(&mut rng).unwrap() {
                Operation::Insert => {
                    // should success
                    let data: u64 = rng.gen();

                    println!(
                        "[{:0>10}] InsertNone: ({:?}, {})",
                        i, not_existing_key, data
                    );
                    assert_eq!(map.insert(&not_existing_key, data), Ok(()));
                    assert_eq!(ref_map.insert(not_existing_key.clone(), data), None);
                }
                Operation::Lookup => {
                    // should fail
                    println!("[{:0>10}] LookupNone: ({:?}, None)", i, not_existing_key);
                    assert_eq!(ref_map.get(&not_existing_key), None);
                    assert_eq!(map.lookup(&not_existing_key), None);
                }
                Operation::Remove => {
                    // should fail
                    println!("[{:0>10}] RemoveNone: ({:?}, Err)", i, not_existing_key);
                    assert_eq!(ref_map.remove(&not_existing_key), None);
                    assert_eq!(map.remove(&not_existing_key), Err(()));
                }
            }
        } else {
            // run operation with existing key
            let existing_key = (*existing_key.unwrap()).clone();

            match ops.choose(&mut rng).unwrap() {
                Operation::Insert => {
                    // should fail
                    let data: u64 = rng.gen();

                    println!("[{:0>10}] InsertSome: ({:?}, {})", i, existing_key, data);
                    assert_eq!(map.insert(&existing_key, data), Err(data));
                }
                Operation::Lookup => {
                    // should success
                    let data = ref_map.get(&existing_key);

                    println!(
                        "[{:0>10}] LookupSome: ({:?}, {})",
                        i,
                        existing_key,
                        data.unwrap()
                    );
                    assert_eq!(map.lookup(&existing_key), data);
                }
                Operation::Remove => {
                    // should success
                    let data = ref_map.remove(&existing_key);

                    println!(
                        "[{:0>10}] RemoveSome: ({:?}, {})",
                        i,
                        existing_key,
                        data.unwrap()
                    );
                    assert_eq!(map.remove(&existing_key).ok(), data);

                    // early stop code if the remove has any problems
                    // for key in ref_map.keys().collect::<Vec<&K>>() {
                    //     assert_eq!(map.lookup(key).is_some(), true, "the key {:?} is not found.", key);
                    // }
                }
            }
        }
    }
}

struct Sequentialized<K, V, M>
where
    K: Eq,
    M: ConcurrentMap<K, V>,
{
    inner: M,
    temp: *const Option<V>,
    _marker: PhantomData<(*const K, V)>,
}

impl<K, V, M> SequentialMap<K, V> for Sequentialized<K, V, M>
where
    K: Eq,
    M: ConcurrentMap<K, V>,
{
    fn new() -> Self {
        let empty: Box<Option<V>> = Box::new(None);

        Self {
            inner: M::new(),
            temp: Box::leak(empty) as *const Option<V>,
            _marker: PhantomData,
        }
    }

    fn insert(&mut self, key: &K, value: V) -> Result<(), V> {
        self.inner.insert(key, value, &pin())
    }

    fn lookup(&self, key: &K) -> Option<&V> {
        let value = self.inner.lookup(key, &pin());

        // HACK: temporarily save the value, and get its reference safely
        unsafe {
            *(self.temp as *mut Option<V>) = value;
            (*self.temp).as_ref()
        }
    }

    fn remove(&mut self, key: &K) -> Result<V, ()> {
        self.inner.remove(key, &pin())
    }
}

pub fn stress_concurrent_as_sequential<K, M>(iter: u64)
where
    K: Ord + Clone + Random + Debug,
    M: ConcurrentMap<K, u64>,
{
    stress_sequential::<K, Sequentialized<K, u64, M>>(iter)
}

#[derive(Clone, Debug)]
struct Log<K, V> {
    start: Instant,
    end: Instant,
    op: Operation,
    key: K,
    // insert: Try inserting (K, V). If success, Ok(V)
    // lookup: Try looking up (K, ). If existing (K, V), Ok(V)
    // remove: Try removing (K, ). If success to remove (K, V), Ok(V)
    result: Result<V, ()>,
}

// LogBunch: (start, end) of Insert, (start, end) of Remove, logs
type LogBunch<K, V> = (Instant, Instant, Instant, Instant, Vec<Log<K, V>>);

fn print_logs<K: Debug>(logs: &Vec<Log<K, u64>>) {
    for log in logs {
        println!("{:?}", log);
    }
}

pub fn stress_concurrent<K, M>(iter: u64, thread_num: u64)
where
    K: Send + Ord + Clone + Random + Debug + Hash,
    M: Sync + ConcurrentMap<K, u64>,
{
    let ops = [Operation::Insert, Operation::Lookup, Operation::Remove];

    let map = M::new();

    let logs = thread::scope(|s| {
        let mut threads = Vec::new();

        for _ in 0..thread_num {
            let t = s.spawn(|_| {
                let mut rng = thread_rng();
                let mut logs = Vec::new();

                for _ in 0..iter {
                    let pin = pin();

                    let key = K::gen(&mut rng);
                    let op = ops.choose(&mut rng).unwrap().clone();

                    let (start, result, end) = match op {
                        Operation::Insert => {
                            let value = u64::gen(&mut rng);
                            let start = Instant::now();
                            let result = match map.insert(&key, value, &pin) {
                                Ok(()) => Ok(value),
                                Err(_) => Err(()),
                            };
                            let end = Instant::now();

                            (start, result, end)
                        }
                        Operation::Lookup => {
                            let start = Instant::now();
                            let result = match map.lookup(&key, &pin) {
                                Some(value) => Ok(value),
                                None => Err(()),
                            };
                            let end = Instant::now();

                            (start, result, end)
                        }
                        Operation::Remove => {
                            let start = Instant::now();
                            let result = map.remove(&key, &pin);
                            let end = Instant::now();

                            (start, result, end)
                        }
                    };

                    let log = Log {
                        start,
                        end,
                        op,
                        key,
                        result,
                    };

                    // println!("{:?} [{:0>10}] {:?}", std::thread::current().id(), i, log);

                    logs.push(log);
                    drop(pin);
                }

                logs
            });

            threads.push(t);
        }

        threads
            .into_iter()
            .map(|h| h.join().unwrap())
            .flatten()
            .collect::<Vec<_>>()
    })
    .unwrap();

    assert_logs(logs);
}

// rearrange logs and check if they are consistent and have no contradiction
fn assert_logs<K: Ord + Hash + Clone + Debug>(logs: Vec<Log<K, u64>>) {
    let mut key_logs = HashMap::new();

    // classify logs by key
    for log in logs {
        key_logs
            .entry(log.key.clone())
            .or_insert_with(|| Vec::new())
            .push(log);
    }

    for (key, mut key_logs) in key_logs {
        // println!("key: {:?}, num: {}", key, key_logs.len());
        key_logs.sort_by(|a, b| a.start.cmp(&b.start));

        let mut value_logs = HashMap::new();

        for log in &key_logs {
            value_logs
                .entry(log.result.clone())
                .or_insert_with(|| Vec::new())
                .push(log.clone());
        }

        let mut error_logs = Vec::new();

        let mut log_bunches: Vec<LogBunch<K, u64>> = Vec::new();
        let mut last_flag = false;
        for (value, mut logs) in value_logs {
            if value == Err(()) {
                // Error logs cannot cause side effect. Therefore, collect all in one place and check correctness
                error_logs = logs;
                continue;
            }

            // make logs like [Insert, ..., Remove]
            let mut fixed_logs = Vec::new();
            let mut remove_log = None;

            for log in logs {
                match log.op {
                    Operation::Insert => fixed_logs.insert(0, log),
                    Operation::Lookup => fixed_logs.push(log),
                    Operation::Remove => remove_log = Some(log),
                }
            }

            if let Some(log) = remove_log {
                fixed_logs.push(log);
            }

            logs = fixed_logs;

            // print_logs(&logs);
            verify_logs(logs.iter().collect::<Vec<_>>());

            // TODO: split bunch into multiple bunches if multiple insert-remove pairs exist.
            let insert = (&logs)
                .into_iter()
                .filter(|x| x.op == Operation::Insert)
                .collect::<Vec<_>>();
            let remove = (&logs)
                .into_iter()
                .filter(|x| x.op == Operation::Remove)
                .collect::<Vec<_>>();

            assert_eq!(
                insert.len(),
                1,
                "On one value, multiple insert is not checked right now."
            );
            assert!(
                remove.len() <= 1,
                "On one value, multiple remove is not checked right now."
            );

            let insert = logs.first().unwrap();

            if remove.len() == 0 {
                // the latest insertion on the key
                if last_flag {
                    println!("Full logs of key: {:?}, value: {:?}", key, value);
                    print_logs(&key_logs);
                    panic!("Cannot multiple insetion on last");
                }

                last_flag = true;
                let last_instant = Instant::now()
                    .checked_add(Duration::from_secs(300))
                    .unwrap();
                log_bunches.push((insert.start, insert.end, last_instant, last_instant, logs));
            } else {
                let remove = logs.last().unwrap();
                log_bunches.push((insert.start, insert.end, remove.start, remove.end, logs));
            }
        }

        if log_bunches.is_empty() {
            // There are only error logs. Therefore, we just check if the log is lookup(error) or remove(error).

            for error_log in error_logs {
                if error_log.op == Operation::Insert {
                    panic!("If there are only error logs, they should be lookup or removal.");
                }
            }

            continue;
        }

        // rearrange batches by correctness
        log_bunches.sort_by(|a, b| a.0.cmp(&b.0));

        let before = log_bunches.len();

        let mut log_bunches = VecDeque::from(log_bunches);
        let mut final_log_bunches = vec![log_bunches.pop_front().unwrap()];

        rearrange_log_bunches(&mut final_log_bunches, &mut log_bunches)
            .expect("Failed to rearrange logs to be correct");

        assert_eq!(before, final_log_bunches.len());

        if last_flag {
            let last_op = &final_log_bunches.last().unwrap().4.last().unwrap().op;
            assert!(*last_op != Operation::Remove);
        }

        // check if the error log can be caused as error
        //
        // the error of insert: ok when its area is overlapped by the success of insertion or the success of removal
        //              lookup: ok when its area is overlapped by the success of removal or the success of insertion
        //              remove: same to the error of lookup
        error_logs.sort_by(|a, b| a.start.cmp(&b.start));

        println!("key: {:?}", key);
        println!("error_logs: {:?}", error_logs);
        println!("log bunches: {:?}", final_log_bunches);

        // check the first range by first log bunch
        {
            let first_log_bunch = final_log_bunches.first().unwrap();

            let mut i = 0;
            while i < error_logs.len() {
                let error_log = &error_logs[i];

                if error_log.start < first_log_bunch.3 {
                    // the error log is overlapped by the range of the bunch
                    match error_log.op {
                        Operation::Insert => {
                            // insert overlapping first_log_bunch's inserting or removing,
                            // or between first_log_bunch's finishing inserting and starting removing
                            if (error_log.start < first_log_bunch.1 && error_log.end > first_log_bunch.0)
                                || (error_log.start < first_log_bunch.3 && error_log.end > first_log_bunch.2)
                                || (error_log.start < first_log_bunch.2 && error_log.end > first_log_bunch.1)
                            {
                                error_logs.remove(i);
                                continue;
                            }
                        }
                        Operation::Lookup | Operation::Remove => {
                            // lookup/remove overlapping or before first_log_bunch's inserting,
                            if error_log.start < first_log_bunch.1 {
                                error_logs.remove(i);
                                continue;
                            }
                        }
                    }
                } else {
                    // it or later error log cannot be overlapped by the range of the two bunches
                    // therefore, break and try on next range
                    break;
                }

                i += 1;
            }
        }

        // check the middle range by the two log bunches
        for bunches in final_log_bunches.windows(2) {
            let old = &bunches[0];
            let new = &bunches[1];
            let (start, end) = (old.1, new.1); // from finishing old inserting to finishing new inserting

            let mut i = 0;
            while i < error_logs.len() {
                let error_log = &error_logs[i];

                if error_log.start < end && error_log.end > start {
                    // the error log is overlapped by the range
                    match error_log.op {
                        Operation::Insert => {
                            // insert overlapping old removal and new insertion,
                            // or between finishing old insertion and starting old removal
                            if error_log.start < old.3
                                || (error_log.start < new.1 && error_log.end > new.0)
                            {
                                error_logs.remove(i);
                                continue;
                            }
                        }
                        Operation::Lookup | Operation::Remove => {
                            // lookup/remove overlapping old removal and new insertion,
                            // or between finishing old removal and starting new insertion
                            if error_log.end < old.2 || error_log.end > new.0 {
                                error_logs.remove(i);
                                continue;
                            }
                        }
                    }
                } else {
                    // it or later error log cannot be overlapped by the range of the two bunches
                    // therefore, break and try on next range
                    break;
                }

                i += 1;
            }
        }

        // check the last range by the log bunch
        {
            let last_log_bunch = final_log_bunches.last().unwrap();

            let mut i = 0;
            while i < error_logs.len() {
                let error_log = &error_logs[i];

                if error_log.start < last_log_bunch.3 {
                    // the error log is overlapped by the range of the bunch
                    match error_log.op {
                        Operation::Insert => {
                            // insert overlapping first_log_bunch's removing,
                            // or between first_log_bunch's finishing inserting and starting removing
                            if (error_log.start < last_log_bunch.3 && error_log.end > last_log_bunch.2)
                                || (error_log.start < last_log_bunch.2 && error_log.end > last_log_bunch.1)
                            {
                                error_logs.remove(i);
                                continue;
                            }
                        }
                        Operation::Lookup | Operation::Remove => {
                            if last_log_bunch.4.last().unwrap().op == Operation::Remove
                                && error_log.end > last_log_bunch.2
                            {
                                // they can be correct if the removal occured, or should be incorrect
                                error_logs.remove(i);
                                continue;
                            }
                        }
                    }
                } else {
                    // it or later error log cannot be overlapped by the range of the two bunches
                    // therefore, break and try on next range
                    break;
                }

                i += 1;
            }
        }

        assert_eq!(error_logs.len(), 0, "Error logs are not fully handled.");

        // merge log bunches into single log
        let logs: Vec<Log<K, u64>> = final_log_bunches
            .into_iter()
            .map(|bunch| bunch.4)
            .flatten()
            .collect();

        assert!(verify_logs(logs.iter().collect::<Vec<_>>()));
    }
}

// rearrange log bunches to be correct
//
// to use this function, please set first element into now_log_bunches by front poping from rest_log_bunches
// use DFS:
// 1) Insert b_1(bunch) into []
// i+1) For [b_1, ..., b_i], try insert b_{i+1}. If failed, insert b_{i+1} moving backward.
//      (ex. try inserting [b_1, ..., b_{i - 3}, b_{i + 1}, b_{i - 2}, b_{i - 1}, b_i])
//      If failed to try all case on [b_1, ..., b_i], go back [b_1, ..., b_{i - 1}] and try inserting b_i on other place.
//      If failed to try all case on the list, the program is incorrect.
fn rearrange_log_bunches<K: Debug, V: Clone + Debug + PartialEq>(
    now_log_bunches: &mut Vec<LogBunch<K, V>>,
    rest_log_bunches: &mut VecDeque<LogBunch<K, V>>,
) -> Result<(), ()> {
    if rest_log_bunches.is_empty() {
        return Ok(());
    }

    if verify_log_bunches(vec![
        now_log_bunches.last().unwrap(),
        rest_log_bunches.front().unwrap(),
    ]) {
        // very good case: just push now log bunch into full logs
        now_log_bunches.push(rest_log_bunches.pop_front().unwrap());

        let result = rearrange_log_bunches(now_log_bunches, rest_log_bunches);

        if result.is_ok() {
            return Ok(());
        }

        rest_log_bunches.push_front(now_log_bunches.pop().unwrap());
    }

    // try to insert it on best place like [i - 1, it, i]
    for i in (0..now_log_bunches.len()).rev() {
        if now_log_bunches[i].3 < rest_log_bunches.front().unwrap().0 {
            // if the target cannot be followed by now_log_bunches[i], it cannot be inserted. So, break.
            break;
        }

        let mut test_bunches = vec![];

        if i >= 1 {
            test_bunches.push(&now_log_bunches[i - 1]);
        }

        test_bunches.push(rest_log_bunches.front().unwrap());
        test_bunches.push(&now_log_bunches[i]);

        if verify_log_bunches(test_bunches) {
            now_log_bunches.insert(i, rest_log_bunches.pop_front().unwrap());

            let result = rearrange_log_bunches(now_log_bunches, rest_log_bunches);

            if result.is_ok() {
                return Ok(());
            }

            rest_log_bunches.push_front(now_log_bunches.pop().unwrap());
        }
    }

    Err(())
}

fn verify_log_bunches<K: Debug, V: Clone + Debug + PartialEq>(
    log_bunches: Vec<&LogBunch<K, V>>,
) -> bool {
    let merged_logs = log_bunches
        .iter()
        .map(|x| &x.4)
        .flatten()
        .collect::<Vec<_>>();
    verify_logs(merged_logs)
}

// verify if the logs have no contradiction on order
fn verify_logs<K: Debug, V: Clone + Debug + PartialEq>(logs: Vec<&Log<K, V>>) -> bool {
    let mut old_log = &logs[0];
    let mut state = verify_state_log(None, &old_log).unwrap();

    for log in logs.iter().skip(1) {
        // the old log should be former or overlapped
        if old_log.start <= log.end {
            if let Ok(new_state) = verify_state_log(state, &log) {
                state = new_state;
                old_log = log;

                continue;
            } else {
                // The log has contradition on data
                return false;
            }
        } else {
            // The log is inconsistent on time
            return false;
        }
    }

    true
}

// verify if the log is correct to set on right next of the state
// if correct, return Ok() with next state
// if not correct, Err(())
fn verify_state_log<K, V: Clone + PartialEq>(
    state: Option<V>,
    log: &Log<K, V>,
) -> Result<Option<V>, ()> {
    match log.op {
        Operation::Insert => {
            if let Some(_) = state.clone() {
                if let Ok(_) = log.result {
                    Err(())
                } else {
                    Ok(state)
                }
            } else {
                if let Ok(v) = log.result.clone() {
                    Ok(Some(v))
                } else {
                    Err(())
                }
            }
        }
        Operation::Lookup => {
            if let Some(s) = state.clone() {
                if let Ok(v) = log.result.clone() {
                    if s == v {
                        Ok(state)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            } else {
                if let Ok(_) = log.result {
                    Err(())
                } else {
                    Ok(state)
                }
            }
        }
        Operation::Remove => {
            if let Some(s) = state.clone() {
                if let Ok(v) = log.result.clone() {
                    if s == v {
                        Ok(None)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            } else {
                if let Ok(_) = log.result {
                    Err(())
                } else {
                    Ok(state)
                }
            }
        }
    }
}
