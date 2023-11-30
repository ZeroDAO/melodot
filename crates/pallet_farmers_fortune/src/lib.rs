// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod pallet {
    use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

    #[pallet::config]
	pub trait frame_system::Config {

    }

    #[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        
    }

    #[pallet::error]
	pub enum Error<T> {

    }

    #[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
		#[pallet::weight(100_000 + T::DbWeight::get().writes(1))]
    }
    pub fn claim(origin: OriginFor<T>, pre_cell: Cell, win_cell: Cell, win_block_num: T::BlockNumber, nonce: u8) -> DispatchResult {
        let _who = ensure_signed(origin)?;
        Ok(())
    }
}

impl<T: Config> Pallet<T> {

}