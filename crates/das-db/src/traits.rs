// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// `DasKv` is a trait representing a key-value store interface.
pub trait DasKv {
    /// Retrieves the value associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - A byte slice representing the key to look up.
    ///
    /// # Returns
    ///
    /// An `Option` containing the value associated with the key if it exists, or `None` if the key is not present.
    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>>;

    /// Sets a value for the given key in the store.
    ///
    /// # Arguments
    ///
    /// * `key` - A byte slice representing the key to set.
    /// * `value` - A byte slice representing the value to associate with the key.
    fn set(&mut self, key: &[u8], value: &[u8]);

    /// Removes the value associated with the given key from the store.
    ///
    /// # Arguments
    ///
    /// * `key` - A byte slice representing the key to remove.
    fn remove(&mut self, key: &[u8]);

    /// Checks if the store contains the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - A byte slice representing the key to check.
    ///
    /// # Returns
    ///
    /// `true` if the key is present in the store, `false` otherwise.
    fn contains(&mut self, key: &[u8]) -> bool;

    /// Compares the current value of the given key with the specified old value and, if they match, sets the new value.
    ///
    /// # Arguments
    ///
    /// * `key` - A byte slice representing the key to compare and set.
    /// * `old_value` - An `Option` containing a byte slice representing the expected current value of the key.
    ///                 If `None`, it expects the key to not be present in the store.
    /// * `new_value` - A byte slice representing the new value to set if the comparison succeeds.
    ///
    /// # Returns
    ///
    /// `true` if the comparison was successful and the value was set, `false` otherwise.
    fn compare_and_set(&mut self, key: &[u8], old_value: Option<&[u8]>, new_value: &[u8]) -> bool;
}
