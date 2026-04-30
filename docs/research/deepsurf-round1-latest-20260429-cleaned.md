# deepSURF Round-1 SERAPH Evaluation

- Generated at: `2026-04-29 22:55:44`
- Workspaces: `/tmp/seraph-deepsurf-round1-latest-20260429`
- Dataset root: `/tmp/deepSURF-main/dataset`
- Work units attempted: `64`
- Phase 3 enabled: `yes`

## Outcome Counts

- `attempted`: `2`
- `bug`: `1`
- `crate_build_failed`: `6`
- `extract_failed`: `6`
- `no_targets`: `15`
- `runtime_error`: `2`
- `validated`: `32`

## Group Counts

- `CRABTREE`: `8`
- `ERASAN`: `27`
- `RUG`: `13`
- `RUSTSAN`: `16`

## Results

| Group | Unit | Unsafe Targets | Top Target | Outcome | Compile/Fix/Smoke | Error |
|-------|------|----------------|------------|---------|-------------------|-------|
| CRABTREE | fixedbitset-0.4.1 | 9 | `api::fixedbitset::FixedBitSet::with_capacity_and_blocks` | `validated` | `ok/ok/ok` | - |
| CRABTREE | integer-encoding-3.0.4 | 1 | `api::integer_encoding::fixed::FixedInt::switch_endianness` | `validated` | `ok/ok/ok` | - |
| CRABTREE | leapfrog-0.2.1 | 1 | `api::leapfrog::util::load_u64_le` | `crate_build_failed` | `failed/failed/skipped` | - |
| CRABTREE | oxidebpf-0.2.3 | 5 | `api::oxidebpf::maps::ArrayMap::new` | `validated` | `ok/ok/ok` | - |
| CRABTREE | roaring-0.10.1 | 1 | `api::roaring::bitmap::RoaringBitmap::rank` | `validated` | `ok/ok/ok` | - |
| CRABTREE | sharded-slab-0.1.4 | 2 | `api::sharded_slab::pool::OwnedRefMut::downgrade` | `validated` | `failed/ok/ok` | - |
| CRABTREE | sparsey-0.7.0 |  | `-` | `extract_failed` | `-/-/-` | extract:failed to extract knowledge from manifest: failed to parse rustdoc JSON: invalid type: map, expected a string at line 1 column 673 |
| CRABTREE | triomphe-0.1.6 | 21 | `api::triomphe::thin_arc::ThinArc::from_raw` | `validated` | `ok/ok/ok` | - |
| ERASAN | algorithmica-0.1.8 | 0 | `-` | `no_targets` | `-/-/-` | - |
| ERASAN | arc-swap-1.0.0 | 2 | `api::arc_swap::ArcSwapAny::load` | `validated` | `failed/ok/ok` | - |
| ERASAN | bumpalo-3.11.0 | 14 | `api::bumpalo::Bump::iter_allocated_chunks` | `validated` | `ok/ok/ok` | - |
| ERASAN | cbox-0.3.0 | 6 | `api::cbox::CBox::as_semi` | `validated` | `ok/ok/ok` | - |
| ERASAN | endian_trait-0.6.0 | 0 | `-` | `no_targets` | `-/-/-` | - |
| ERASAN | futures-0.3.3-futures-task | 1 | `api::futures_task::future_obj::LocalFutureObj::into_future_obj` | `validated` | `failed/ok/ok` | - |
| ERASAN | futures-0.3.5-futures-task | 1 | `api::futures_task::future_obj::LocalFutureObj::into_future_obj` | `validated` | `failed/ok/ok` | - |
| ERASAN | http-0.1.19 | 6 | `api::http::header::name::HeaderName::from_bytes` | `crate_build_failed` | `failed/failed/skipped` | - |
| ERASAN | insert_many-0.1.1 | 0 | `-` | `no_targets` | `-/-/-` | - |
| ERASAN | lru-0.6.6 | 9 | `api::lru::LruCache::iter` | `validated` | `ok/ok/ok` | - |
| ERASAN | nano_arena-0.5.2 | 1 | `api::nano_arena::Arena::split_at` | `validated` | `ok/ok/ok` | - |
| ERASAN | ordnung-0.0.1 | 3 | `api::ordnung::Map::get` | `validated` | `ok/ok/ok` | - |
| ERASAN | pnet_packet-0.26.0 |  | `-` | `extract_failed` | `-/-/-` | extract:  thread caused non-unwinding panic. aborting. |
| ERASAN | qwutils-0.3.0 | 0 | `-` | `no_targets` | `-/-/-` | - |
| ERASAN | rdiff-0.1.2 | 0 | `-` | `no_targets` | `-/-/-` | - |
| ERASAN | rusqlite-0.26.1 | 10 | `api::rusqlite::OpenFlags::from_bits_unchecked` | `validated` | `ok/ok/ok` | - |
| ERASAN | secp256k1-0.22.0 |  | `-` | `extract_failed` | `-/-/-` | extract:failed to extract knowledge from manifest: failed to parse rustdoc JSON: invalid type: map, expected a string at line 1 column 640256 |
| ERASAN | simple-slab-0.3.2 | 2 | `api::simple_slab::Slab::with_capacity` | `validated` | `ok/ok/ok` | - |
| ERASAN | slice-deque-0.3.0 | 30 | `api::slice_deque::SliceDeque::move_head_unchecked` | `crate_build_failed` | `failed/failed/skipped` | - |
| ERASAN | smallvec-0.6.6 | 13 | `api::smallvec::SmallVec::drain` | `validated` | `ok/ok/ok` | - |
| ERASAN | smallvec-1.6.0 | 14 | `api::smallvec::SmallVec::drain` | `validated` | `ok/ok/ok` | - |
| ERASAN | stack_dst-0.6.0 | 7 | `api::stack_dst::value::ValueA::new_raw` | `validated` | `failed/ok/ok` | - |
| ERASAN | stackvector-1.0.8 | 10 | `api::stackvector::StackVec::drain` | `runtime_error` | `ok/ok/bug` | - |
| ERASAN | string-interner-0.7.0 | 0 | `-` | `no_targets` | `-/-/-` | - |
| ERASAN | through-0.1.0 | 0 | `-` | `no_targets` | `-/-/-` | - |
| ERASAN | tokio-1.24.1 | 11 | `api::tokio::io::read_buf::ReadBuf::assume_init` | `validated` | `ok/ok/ok` | - |
| ERASAN | toodee-0.2.4 | 9 | `api::toodee::ops::TooDeeOps::get_unchecked` | `validated` | `ok/ok/ok` | - |
| RUG | bincode-2.0.0-rc.3 |  | `-` | `extract_failed` | `-/-/-` | extract:failed to extract knowledge from manifest: failed to parse rustdoc JSON: invalid type: map, expected a string at line 1 column 192817 |
| RUG | chrono-0.5.0-alpha.1 | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | crc32fast-1.3.2 | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | hashes-blake2 | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | hashes-sha2 | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | itoa-1.0.6 | 1 | `api::itoa::Buffer::format` | `validated` | `ok/ok/ok` | - |
| RUG | nom-7.1.2 | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | num-traits-0.2.15 | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | ryu-1.0.13 | 3 | `api::ryu::pretty::format32` | `validated` | `ok/ok/ok` | - |
| RUG | semver-1.0.17 | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | serde_json-1.0.96 | 2 | `api::serde_json::ser::to_string` | `crate_build_failed` | `failed/failed/skipped` | - |
| RUG | time | 0 | `-` | `no_targets` | `-/-/-` | - |
| RUG | uuid-1.4.0 | 5 | `api::uuid::Uuid::from_bytes_ref` | `validated` | `ok/ok/ok` | - |
| RUSTSAN | arenavec-0.1.1 | 8 | `api::arenavec::common::SliceVec::split_off` | `runtime_error` | `ok/ok/bug` | - |
| RUSTSAN | array-queue-0.3.3 | 3 | `api::array_queue::array_queue::ArrayQueue::new` | `validated` | `ok/ok/ok` | - |
| RUSTSAN | base64-0.5.1 | 1 | `api::base64::encode_config_buf` | `validated` | `failed/ok/ok` | - |
| RUSTSAN | bumpalo-3.2.0 | 8 | `api::bumpalo::Bump::with_capacity` | `validated` | `ok/ok/ok` | - |
| RUSTSAN | chttp-0.1.2 |  | `-` | `extract_failed` | `-/-/-` | extract:  error occurred in cc-rs: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-fPIC" "-g" "-gdwarf-4" "-fno-omit-frame-pointer" "-m64" "-I" "curl/lib" "-I" "curl/include" "-I" "/usr/include" "-I" "/usr/include" "-w" "-fvisibility=hidden" "-DBUILDING_LIBCURL" "-DCURL_DISABLE_DICT" "-DCURL_DISABLE_GOPHER" "-DCURL_DISABLE_ |
| RUSTSAN | flatbuffers-0.8.0 |  | `-` | `extract_failed` | `-/-/-` | extract:failed to extract knowledge from manifest: failed to parse rustdoc JSON: invalid type: map, expected a string at line 1 column 407276 |
| RUSTSAN | generic-array-0.13.2 | 1 | `api::generic_array::GenericArray::from_exact_iter` | `validated` | `failed/ok/ok` | - |
| RUSTSAN | id-map-0.2.1 | 4 | `api::id_map::IdMap::into_iter` | `bug` | `failed/ok/bug` | - |
| RUSTSAN | safe-transmute-0.10.0 | 28 | `api::safe_transmute::bool::guarded_transmute_bool_pedantic` | `validated` | `failed/ok/ok` | - |
| RUSTSAN | scratchpad-1.3.0 | 31 | `api::scratchpad::allocation::Allocation::concat_unchecked` | `crate_build_failed` | `failed/failed/skipped` | - |
| RUSTSAN | sized-chunks-0.6.2 | 26 | `api::sized_chunks::sparse_chunk::SparseChunk::get_unchecked` | `attempted` | `failed/failed/skipped` | - |
| RUSTSAN | slice-deque-0.1.15 | 29 | `api::slice_deque::SliceDeque::move_head_unchecked` | `crate_build_failed` | `failed/failed/skipped` | - |
| RUSTSAN | smallvec-0.6.1 | 12 | `api::smallvec::SmallVec::drain` | `validated` | `failed/ok/ok` | - |
| RUSTSAN | smallvec-0.6.3 | 12 | `api::smallvec::SmallVec::drain` | `validated` | `ok/ok/ok` | - |
| RUSTSAN | stack-0.3.0 | 12 | `api::stack::array_vec::ArrayVec::into_inner` | `attempted` | `failed/failed/skipped` | - |
| RUSTSAN | sys-info-0.7.0 | 10 | `api::sys_info::disk_info` | `validated` | `ok/ok/ok` | - |
