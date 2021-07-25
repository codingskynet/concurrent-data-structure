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

                for i in 0..iter {
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

// bug: if the bunch of operations are moved to near future and it causes inconsistency,
// this alogorithm cannot rearrange well.
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
        println!("key: {:?}, num: {}", key, key_logs.len());
        key_logs.sort_by(|a, b| a.start.cmp(&b.start));

        let mut value_logs = HashMap::new();

        for log in &key_logs {
            value_logs
                .entry(log.result.clone())
                .or_insert_with(|| Vec::new())
                .push(log.clone());
        }

        let mut error_logs: Vec<Log<K, u64>>;

        // LogBunch: (start, end) of Insert, (start, end) of Remove, logs
        type LogBunch<K> = (Instant, Instant, Instant, Instant, Vec<Log<K, u64>>);
        let mut log_bunches: Vec<LogBunch<K>> = Vec::new();
        let mut last_flag = false;
        for (value, mut logs) in value_logs {
            if value == Err(()) {
                // Error logs cannot cause side effect. Therefore, collect all in one place and interleave them into correct logs.
                error_logs = logs;
                continue;
            }

            for i in 0..logs.len() {
                match logs[i].op {
                    Operation::Insert => {
                        let insert_log = logs.remove(i);
                        logs.insert(0, insert_log);
                    }
                    Operation::Lookup => {}
                    Operation::Remove => {
                        let remove_log = logs.remove(i);
                        logs.push(remove_log);
                    }
                }
            }

            println!("value: {}", value.unwrap());
            print_logs(&logs);
            verify_logs(&logs);

            // TODO: split bunch into multiple bunches if multiple insert-remove pairs exist.
            let insert = (&logs).into_iter().filter(|x| x.op == Operation::Insert).collect::<Vec<_>>();
            let remove = (&logs).into_iter().filter(|x| x.op == Operation::Remove).collect::<Vec<_>>();

            assert_eq!(
                insert.len(), 1,
                "On one value, multiple insert is not checked right now."
            );
            assert!(
                remove.len() <= 1,
                "On one value, multiple remove is not checked right now."
            );

            let insert = logs.first().unwrap();

            // the latest insertion on the key
            if remove.len() == 0 {
                if last_flag {
                    println!("Full logs");
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

        // merge log bunches into single log
        log_bunches.sort_by(|a, b| a.0.cmp(&b.0));

        // assert_eq!(failed_logs.len(), 0);

        // verify_logs(logs);
    }
}

// verify if the logs have no contradiction on order
fn verify_logs<K: Debug>(logs: &Vec<Log<K, u64>>) {
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
                panic!(
                    "The log has contradition on data. old: {:?}, new: {:?}",
                    old_log, log
                );
            }
        } else {
            panic!(
                "The log is inconsistent on time. old: {:?}, new: {:?}",
                old_log, log
            );
        }
    }
}

// verify if the log is correct to set on right next of the state
// if correct, return Ok() with next state
// if not correct, Err(())
fn verify_state_log<K>(state: Option<u64>, log: &Log<K, u64>) -> Result<Option<u64>, ()> {
    match log.op {
        Operation::Insert => {
            if let Some(_) = state {
                if let Ok(_) = log.result {
                    Err(())
                } else {
                    Ok(state)
                }
            } else {
                if let Ok(v) = log.result {
                    Ok(Some(v))
                } else {
                    Err(())
                }
            }
        }
        Operation::Lookup => {
            if let Some(s) = state {
                if let Ok(v) = log.result {
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
            if let Some(s) = state {
                if let Ok(v) = log.result {
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
