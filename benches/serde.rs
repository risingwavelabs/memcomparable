// Copyright 2022 Singularity Data
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use criterion::{criterion_group, criterion_main, Criterion};
use memcomparable::{Decimal, Deserializer, Serializer};

criterion_group!(benches, decimal);
criterion_main!(benches);

fn decimal(c: &mut Criterion) {
    // generate decimals
    let mut decimals = vec![Decimal::NaN, Decimal::NegInf, Decimal::Inf, Decimal::ZERO];
    for _ in 0..10 {
        decimals.push(Decimal::Normalized(rand::random()));
    }

    c.bench_function("serialize_decimal", |b| {
        b.iter(|| {
            let mut ser = Serializer::new(vec![]);
            for d in &decimals {
                ser.serialize_decimal(*d).unwrap();
            }
        })
    });

    c.bench_function("deserialize_decimal", |b| {
        let encodings = decimals
            .iter()
            .map(|d| {
                let mut ser = Serializer::new(vec![]);
                ser.serialize_decimal(*d).unwrap();
                ser.into_inner()
            })
            .collect::<Vec<_>>();
        b.iter(|| {
            for encoding in &encodings {
                Deserializer::new(encoding.as_slice())
                    .deserialize_decimal()
                    .unwrap();
            }
        })
    });
}
