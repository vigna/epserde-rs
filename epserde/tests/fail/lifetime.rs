/*
 * SPDX-FileCopyrightText: 2025 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use epserde::prelude::*;

#[derive(Epserde)]
struct Data<'a>(std::marker::PhantomData<&'a ()>);

fn main() {}
