// use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
// use bench_sig::multisig::MultisigSettings;

// const FAULTY_VALUES: [u16; 4] = [10, 30, 50, 70];

// fn multisig_bench(c: &mut Criterion) {
//     let mut group = c.benchmark_group("multi-sig");
//     for num_faulty in FAULTY_VALUES.iter().map(|x| *x as usize) {
//         let settings = multisig::MultisigSettings {
//             system_size: 3 * num_faulty + 1,
//             threshold: 2 * num_faulty + 1,
//         };
//         let message = b"message to sign";

//         let mut setup: Option<multisig::MultisigSetup> = None;
//         let mut sign: Option<multisig::MultisigSign> = None;

//         let id = BenchmarkId::new("setup", num_faulty);
//         group.bench_with_input(id, &num_faulty, |b, _| {
//             b.iter(|| setup = Some(multisig::setup(&settings)));
//         });
//         let id = BenchmarkId::new("sign", num_faulty);
//         group.bench_with_input(id, &num_faulty, |b, _| {
//             b.iter(|| sign = Some(multisig::sign_message(&setup.as_ref().unwrap(), message)));
//         });
//         let id = BenchmarkId::new("verify", num_faulty);
//         group.bench_with_input(id, &num_faulty, |b, _| {
//             b.iter(|| {
//                 multisig::aggregate_verify(
//                     &settings,
//                     setup.as_ref().unwrap(),
//                     sign.as_ref().unwrap(),
//                     message,
//                 )
//             });
//         });
//     }
//     group.finish();
// }

// fn benchmarks(c: &mut Criterion) {
//     multisig_bench(c);
// }

// criterion_group!(benches, benchmarks);
// criterion_main!(benches);
