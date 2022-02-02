// Copyright 2022 Alex Chi. Licensed under Apache-2.0.

//! Implements string functions for [`Array`] types

#![allow(dead_code)]

pub fn str_contains(i1: &str, i2: &str) -> bool {
    i1.contains(i2)
}
