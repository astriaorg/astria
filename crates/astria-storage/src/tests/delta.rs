/*
#[tokio::test]
async fn garden_of_forking_paths() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let storage = TempStorage::new().await?;

    let mut state_init = storage.latest_snapshot();

    // TODO: do we still want to have StateTransaction ?
    // what if we just made StateDelta be StateTransaction ?
    // what are the downsides? forced allocation for range queries?
    // where do we get the events out?
    let mut tx = state_init.begin_transaction();
    tx.put_raw("base".to_owned(), b"base".to_vec());
    tx.apply();
    storage.commit(state_init).await?;

    let mut state = storage.latest_snapshot();
    let mut tx = state.begin_transaction();

    // We can create a StateDelta from a borrow, it will take ownership of the borrow while the family is live
    let mut delta = StateDelta::new(&mut tx);
    delta.put_raw("delta".to_owned(), b"delta".to_vec());

    // We can also nest StateDeltas -- unlike fork, this will only flatten down to the nesting point.
    let mut d2 = StateDelta::new(&mut delta);

    let mut delta_a = d2.fork();
    let mut delta_b = d2.fork();
    delta_a.put_raw("delta".to_owned(), b"delta_a".to_vec());
    delta_b.put_raw("delta".to_owned(), b"delta_b".to_vec());
    let mut delta_a_base = delta_a.fork();
    let mut delta_b_base = delta_b.fork();
    delta_a_base.delete("base".to_owned());
    delta_b_base.delete("base".to_owned());

    assert_eq!(delta_a.get_raw("base").await?, Some(b"base".to_vec()));
    assert_eq!(delta_a.get_raw("base").await?, Some(b"base".to_vec()));
    assert_eq!(delta_a_base.get_raw("base").await?, None);
    assert_eq!(delta_b_base.get_raw("base").await?, None);

    assert_eq!(delta_a.get_raw("delta").await?, Some(b"delta_a".to_vec()));
    assert_eq!(
        delta_a_base.get_raw("delta").await?,
        Some(b"delta_a".to_vec())
    );
    assert_eq!(delta_b.get_raw("delta").await?, Some(b"delta_b".to_vec()));
    assert_eq!(
        delta_b_base.get_raw("delta").await?,
        Some(b"delta_b".to_vec())
    );

    // Pick one we like and apply it, releasing the &mut delta reference...
    // Note: flattens delta_b_base -> delta_b -> delta and stops!
    delta_b_base.apply();
    // ... so we can read from delta again.
    assert_eq!(delta.get_raw("base").await?, None);
    assert_eq!(delta.get_raw("delta").await?, Some(b"delta_b".to_vec()));

    delta.apply();
    tx.apply();
    storage.commit(state).await?;

    let state = storage.latest_snapshot();
    assert_eq!(state.get_raw("base").await?, None);
    assert_eq!(state.get_raw("delta").await?, Some(b"delta_b".to_vec()));

    Ok(())
}

#[tokio::test]
async fn simple_flow() -> anyhow::Result<()> {
    //tracing_subscriber::fmt::init();
    let tmpdir = tempfile::tempdir()?;

    // Initialize an empty Storage in the new directory
    let storage = Storage::load(tmpdir.path().to_owned()).await?;

    // Version -1 to Version 0 writes
    //
    // tx00: test => test
    // tx00: c/aa => 0 [object store]
    // tx00: c/ab => 1 [object store]
    // tx00: c/ac => 2 [object store]
    // tx00: c/ad => 3 [object store]
    // tx00: iA => A [nonverifiable store]
    // tx00: iC => C [nonverifiable store]
    // tx00: iF => F [nonverifiable store]
    // tx00: iD => D [nonverifiable store]
    // tx01: a/aa => aa
    // tx01: a/aaa => aaa
    // tx01: a/ab => ab
    // tx01: a/z  => z
    // tx01: c/ab => 10 [object store]
    // tx01: c/ac => [deleted] [object store]
    //
    // Version 0 to Version 1 writes
    // tx10: test => [deleted]
    // tx10: a/aaa => [deleted]
    // tx10: a/c => c
    // tx10: iB => B [nonverifiable store]
    // tx11: a/ab => ab2
    // tx11: iD => [deleted] nonverifiable store]

    let mut state_init = StateDelta::new(storage.latest_snapshot());
    // Check that reads on an empty state return Ok(None)
    assert_eq!(state_init.get_raw("test").await?, None);
    assert_eq!(state_init.get_raw("a/aa").await?, None);

    // Create tx00
    let mut tx00 = StateDelta::new(&mut state_init);
    tx00.put_raw("test".to_owned(), b"test".to_vec());
    tx00.object_put("c/aa", 0u64);
    tx00.object_put("c/ab", 1u64);
    tx00.object_put("c/ac", 2u64);
    tx00.object_put("c/ad", 3u64);
    tx00.nonverifiable_put_raw(b"iA".to_vec(), b"A".to_vec());
    tx00.nonverifiable_put_raw(b"iC".to_vec(), b"C".to_vec());
    tx00.nonverifiable_put_raw(b"iF".to_vec(), b"F".to_vec());
    tx00.nonverifiable_put_raw(b"iD".to_vec(), b"D".to_vec());

    // Check reads against tx00:
    //     This is present in tx00
    assert_eq!(tx00.get_raw("test").await?, Some(b"test".to_vec()));
    //     This is missing in tx00 and state_init and tree is empty
    assert_eq!(tx00.get_raw("a/aa").await?, None);
    //     Present in tx00 object store
    assert_eq!(tx00.object_get("c/aa"), Some(0u64));
    assert_eq!(tx00.object_get("c/ab"), Some(1u64));
    assert_eq!(tx00.object_get("c/ac"), Some(2u64));
    assert_eq!(tx00.object_get("c/ad"), Some(3u64));
    //     Present in tx00 object store but requested with wrong type
    assert_eq!(tx00.object_get::<bool>("c/aa"), None);
    //     Missing in tx00 object store
    assert_eq!(tx00.object_get::<bool>("nonexist"), None);
    //     Nonconsensus range checks
    let mut range = tx00.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iD".to_vec(), b"D".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Now apply the transaction to state_init
    tx00.apply();
    assert_eq!(state_init.get_raw("test").await?, Some(b"test".to_vec()));
    assert_eq!(state_init.get_raw("a/aa").await?, None);
    //     Present in state_init object store
    assert_eq!(state_init.object_get("c/aa"), Some(0u64));
    assert_eq!(state_init.object_get("c/ab"), Some(1u64));
    assert_eq!(state_init.object_get("c/ac"), Some(2u64));
    assert_eq!(state_init.object_get("c/ad"), Some(3u64));
    //     Present in state_init object store but requested with wrong type
    assert_eq!(state_init.object_get::<bool>("c/aa"), None);
    //     Missing in state_init object store
    assert_eq!(state_init.object_get::<bool>("nonexist"), None);
    //     Nonconsensus range checks
    let mut range = state_init.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iD".to_vec(), b"D".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Create a transaction writing the other keys.
    let mut tx01 = StateDelta::new(&mut state_init);
    tx01.put_raw("a/aa".to_owned(), b"aa".to_vec());
    tx01.put_raw("a/aaa".to_owned(), b"aaa".to_vec());
    tx01.put_raw("a/ab".to_owned(), b"ab".to_vec());
    tx01.put_raw("a/z".to_owned(), b"z".to_vec());
    tx01.object_put("c/ab", 10u64);
    tx01.object_delete("c/ac");

    // Check reads against tx01:
    //    This is missing in tx01 and reads through to state_init
    assert_eq!(tx01.get_raw("test").await?, Some(b"test".to_vec()));
    //    This is present in tx01
    assert_eq!(tx01.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(tx01.get_raw("a/aaa").await?, Some(b"aaa".to_vec()));
    assert_eq!(tx01.get_raw("a/ab").await?, Some(b"ab".to_vec()));
    assert_eq!(tx01.get_raw("a/z").await?, Some(b"z".to_vec()));
    //    This is missing in tx01 and in state_init
    assert_eq!(tx01.get_raw("a/c").await?, None);
    let mut range = tx01.prefix_raw("a/");
    let mut range_keys = tx01.prefix_keys("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range_keys.next().await.transpose()?,
        Some("a/aa".to_owned())
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aaa".to_owned(), b"aaa".to_vec()))
    );
    assert_eq!(
        range_keys.next().await.transpose()?,
        Some("a/aaa".to_owned())
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab".to_vec()))
    );
    assert_eq!(
        range_keys.next().await.transpose()?,
        Some("a/ab".to_owned())
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range_keys.next().await.transpose()?, Some("a/z".to_owned()));
    assert_eq!(range.next().await.transpose()?, None);
    assert_eq!(range_keys.next().await.transpose()?, None);
    std::mem::drop(range);
    std::mem::drop(range_keys);

    // Now apply the transaction to state_init
    tx01.apply();

    // Check reads against state_init:
    //    This is present in state_init
    assert_eq!(state_init.get_raw("test").await?, Some(b"test".to_vec()));
    assert_eq!(state_init.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(state_init.get_raw("a/aaa").await?, Some(b"aaa".to_vec()));
    assert_eq!(state_init.get_raw("a/ab").await?, Some(b"ab".to_vec()));
    assert_eq!(state_init.get_raw("a/z").await?, Some(b"z".to_vec()));
    //    This is missing in state_init
    assert_eq!(state_init.get_raw("a/c").await?, None);
    let mut range = state_init.prefix_raw("a/");
    let mut range_keys = state_init.prefix_keys("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range_keys.next().await.transpose()?,
        Some("a/aa".to_owned())
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aaa".to_owned(), b"aaa".to_vec()))
    );
    assert_eq!(
        range_keys.next().await.transpose()?,
        Some("a/aaa".to_owned())
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab".to_vec()))
    );
    assert_eq!(
        range_keys.next().await.transpose()?,
        Some("a/ab".to_owned())
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range_keys.next().await.transpose()?, Some("a/z".to_owned()));
    assert_eq!(range.next().await.transpose()?, None);
    assert_eq!(range_keys.next().await.transpose()?, None);
    std::mem::drop(range);
    std::mem::drop(range_keys);

    // Now commit state_init to storage
    storage.commit_delta(state_init).await?;

    // Now we have version 0.
    let mut state0 = StateDelta::new(storage.latest_snapshot());
    //assert_eq!(state0.version(), 0);
    // Check reads against state0:
    //    This is missing in state0 and present in JMT
    assert_eq!(state0.get_raw("test").await?, Some(b"test".to_vec()));
    assert_eq!(state0.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(state0.get_raw("a/aaa").await?, Some(b"aaa".to_vec()));
    assert_eq!(state0.get_raw("a/ab").await?, Some(b"ab".to_vec()));
    assert_eq!(state0.get_raw("a/z").await?, Some(b"z".to_vec()));
    //    This is missing in state0 and missing in JMT
    assert_eq!(state0.get_raw("a/c").await?, None);
    let mut range = state0.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aaa".to_owned(), b"aaa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);
    //     Nonconsensus range checks
    let mut range = state0.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iD".to_vec(), b"D".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Start building a transaction
    let mut tx10 = StateDelta::new(&mut state0);
    tx10.delete("test".to_owned());
    tx10.delete("a/aaa".to_owned());
    tx10.put_raw("a/c".to_owned(), b"c".to_vec());
    tx10.nonverifiable_put_raw(b"iB".to_vec(), b"B".to_vec());

    // Check reads against tx10:
    //    This is deleted in tx10, missing in state0, present in JMT
    assert_eq!(tx10.get_raw("test").await?, None);
    assert_eq!(tx10.get_raw("a/aaa").await?, None);
    //    This is missing in tx10, missing in state0, present in JMT
    assert_eq!(tx10.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(tx10.get_raw("a/ab").await?, Some(b"ab".to_vec()));
    assert_eq!(tx10.get_raw("a/z").await?, Some(b"z".to_vec()));
    //    This is present in tx10, missing in state0, missing in JMT
    assert_eq!(tx10.get_raw("a/c").await?, Some(b"c".to_vec()));
    let mut range = tx10.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/c".to_owned(), b"c".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);
    //     Nonconsensus range checks
    let mut range = tx10.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iB".to_vec(), b"B".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iD".to_vec(), b"D".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Apply tx10 to state0
    tx10.apply();

    // Check reads against state0
    //    This is deleted in state0, present in JMT
    assert_eq!(state0.get_raw("test").await?, None);
    assert_eq!(state0.get_raw("a/aaa").await?, None);
    //    This is missing in state0, present in JMT
    assert_eq!(state0.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(state0.get_raw("a/ab").await?, Some(b"ab".to_vec()));
    assert_eq!(state0.get_raw("a/z").await?, Some(b"z".to_vec()));
    //    This is present in state0, missing in JMT
    assert_eq!(state0.get_raw("a/c").await?, Some(b"c".to_vec()));
    let mut range = state0.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/c".to_owned(), b"c".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Start building another transaction
    let mut tx11 = StateDelta::new(&mut state0);
    tx11.put_raw("a/ab".to_owned(), b"ab2".to_vec());
    tx11.nonverifiable_delete(b"iD".to_vec());

    // Check reads against tx11:
    //    This is present in tx11, missing in state0, present in JMT
    assert_eq!(tx11.get_raw("a/ab").await?, Some(b"ab2".to_vec()));
    //    This is missing in tx11, deleted in state0, present in JMT
    assert_eq!(tx11.get_raw("test").await?, None);
    assert_eq!(tx11.get_raw("a/aaa").await?, None);
    //    This is missing in tx11, missing in state0, present in JMT
    assert_eq!(tx11.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(tx11.get_raw("a/z").await?, Some(b"z".to_vec()));
    //    This is missing in tx10, present in state0, missing in JMT
    assert_eq!(tx11.get_raw("a/c").await?, Some(b"c".to_vec()));
    let mut range = tx11.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab2".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/c".to_owned(), b"c".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);
    //     Nonconsensus range checks
    let mut range = tx11.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iB".to_vec(), b"B".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Apply tx11 to state0
    tx11.apply();

    // Check reads against state0
    //    This is deleted in state0, present in JMT
    assert_eq!(state0.get_raw("test").await?, None);
    assert_eq!(state0.get_raw("a/aaa").await?, None);
    //    This is missing in state0, present in JMT
    assert_eq!(state0.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(state0.get_raw("a/z").await?, Some(b"z".to_vec()));
    //    This is present in state0, missing in JMT
    assert_eq!(state0.get_raw("a/c").await?, Some(b"c".to_vec()));
    //    This is present in state0, present in JMT
    assert_eq!(state0.get_raw("a/ab").await?, Some(b"ab2".to_vec()));
    let mut range = state0.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab2".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/c".to_owned(), b"c".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);
    let mut range = state0.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iB".to_vec(), b"B".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Create another fork of state 0 while we've edited the first one but before we commit.
    let state0a = storage.latest_snapshot();
    assert_eq!(state0a.version(), 0);

    // Commit state0 as state1.
    storage.commit_delta(state0).await?;

    let state1 = storage.latest_snapshot();
    assert_eq!(state1.version(), 1);

    // Check reads against state1
    assert_eq!(state1.get_raw("test").await?, None);
    assert_eq!(state1.get_raw("a/aaa").await?, None);
    assert_eq!(state1.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(state1.get_raw("a/ab").await?, Some(b"ab2".to_vec()));
    assert_eq!(state1.get_raw("a/z").await?, Some(b"z".to_vec()));
    assert_eq!(state1.get_raw("a/c").await?, Some(b"c".to_vec()));
    let mut range = state1.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab2".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/c".to_owned(), b"c".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);
    let mut range = state1.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iB".to_vec(), b"B".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Check reads against state0a
    assert_eq!(state0a.get_raw("test").await?, Some(b"test".to_vec()));
    assert_eq!(state0a.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(state0a.get_raw("a/aaa").await?, Some(b"aaa".to_vec()));
    assert_eq!(state0a.get_raw("a/ab").await?, Some(b"ab".to_vec()));
    assert_eq!(state0a.get_raw("a/z").await?, Some(b"z".to_vec()));
    assert_eq!(state0a.get_raw("a/c").await?, None);
    let mut range = state0a.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aaa".to_owned(), b"aaa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);
    //     Nonconsensus range checks
    let mut range = state0a.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iD".to_vec(), b"D".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    // Now, check that closing and reloading works.

    // First, be sure to explicitly drop anything keeping a reference to the
    // RocksDB instance:
    std::mem::drop(storage);
    // std::mem::drop(state0); // consumed in commit()
    std::mem::drop(state0a);
    std::mem::drop(state1);

    // Now reload the storage from the same directory...
    let storage_a = Storage::load(tmpdir.path().to_owned()).await?;
    let state1a = storage_a.latest_snapshot();

    // Check that we reload at the correct version ...
    assert_eq!(state1a.version(), 1);

    // Check reads against state1a after reloading the DB
    assert_eq!(state1a.get_raw("test").await?, None);
    assert_eq!(state1a.get_raw("a/aaa").await?, None);
    assert_eq!(state1a.get_raw("a/aa").await?, Some(b"aa".to_vec()));
    assert_eq!(state1a.get_raw("a/ab").await?, Some(b"ab2".to_vec()));
    assert_eq!(state1a.get_raw("a/z").await?, Some(b"z".to_vec()));
    assert_eq!(state1a.get_raw("a/c").await?, Some(b"c".to_vec()));
    let mut range = state1a.prefix_raw("a/");
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/aa".to_owned(), b"aa".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/ab".to_owned(), b"ab2".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/c".to_owned(), b"c".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some(("a/z".to_owned(), b"z".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);
    //     Nonconsensus range checks
    let mut range = state1a.nonverifiable_prefix_raw(b"i");
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iA".to_vec(), b"A".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iB".to_vec(), b"B".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iC".to_vec(), b"C".to_vec()))
    );
    assert_eq!(
        range.next().await.transpose()?,
        Some((b"iF".to_vec(), b"F".to_vec()))
    );
    assert_eq!(range.next().await.transpose()?, None);
    std::mem::drop(range);

    Ok(())
}

 */
