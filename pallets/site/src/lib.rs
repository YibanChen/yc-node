//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::inherent::Vec;
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement},
    transactional, Parameter,
};

//use frame_support::{inherent::Vec, pallet_prelude::*, Parameter}, traits::{Currency};
use frame_system::ensure_signed;
pub use pallet::*;
use sp_runtime::{
    traits::{CheckedAdd, One},
    ArithmeticError,
};
use sp_std::result::Result;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Site {
    ipfs_cid: Vec<u8>,
    site_name: Vec<u8>,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::OriginFor;
    use sp_runtime::traits::AtLeast32BitUnsigned;
    use sp_runtime::traits::Bounded;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type SiteIndex: Parameter + AtLeast32BitUnsigned + Bounded + Default + Copy;
    }

    /// Stores all the sites. Key is (T::AccountId, T::SiteIndex).
    #[pallet::storage]
    #[pallet::getter(fn sites)]
    pub type Sites<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::SiteIndex,
        Site,
        OptionQuery,
    >;

    /// Stores the next site Id.
    #[pallet::storage]
    #[pallet::getter(fn next_site_id)]
    pub type NextSiteId<T: Config> = StorageValue<_, T::SiteIndex, ValueQuery>;

    /// Get price.
    #[pallet::storage]
    #[pallet::getter(fn site_prices)]
    pub type SitePrices<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SiteIndex, T::Balance, OptionQuery>;

    /// Get site names.
    #[pallet::storage]
    #[pallet::getter(fn site_names)]
    pub type SiteNames<T: Config> = StorageMap<_, Blake2_128Concat, Site, T::Balance, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    // #[pallet::metadata(T::AccountId = "AccountId", T::SiteIndex = "SiteIndex", Option<T::Balance> = "Option<Balance>")]
    #[pallet::metadata(
		T::AccountId = "AccountId", T::SiteIndex = "SiteIndex", Option<T::Balance> = "Option<Balance>", T::Balance = "Balance",
	)]
    /// All events that can be emitted by Pallet function.
    pub enum Event<T: Config> {
        /// A site is created. \[owner, site_id, site\]
        SiteCreated(T::AccountId, T::SiteIndex, Site),
        /// A site is transfer. \[owner, receiver, site_id\]
        SiteTransferred(T::AccountId, T::AccountId, T::SiteIndex),
        /// A site is burned. \[owner, site_id\]
        SiteBurned(T::AccountId, T::SiteIndex),
        /// The price for a site is updated. \[owner, site_id, price\]
        SitePriceUpdated(T::AccountId, T::SiteIndex, Option<T::Balance>),
        /// A site is sold. \[old_owner, new_owner, site_id, price\]
        SiteSold(T::AccountId, T::AccountId, T::SiteIndex, T::Balance),
    }

    #[pallet::error]
    /// All errors that can be returned by the Pallet function.
    pub enum Error<T> {
        /// The SiteId is invalid and/or doesn't exist.
        InvalidSiteId,
        NotOwner,
        NotForSale,
        PriceTooLow,
        BuyFromSelf,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    /// Contains all user-facing functions.
    impl<T: Config> Pallet<T> {
        /// Create a new Site
        #[pallet::weight(1000)]
        pub fn create(
            origin: OriginFor<T>,
            ipfs_cid: Vec<u8>,
            site_name: Vec<u8>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let site_id = Self::get_next_note_id()?;

            // Create new Site

            let site = Site {
                ipfs_cid: ipfs_cid.clone(),
                site_name: site_name.clone(),
            };

            Sites::<T>::insert(&sender, site_id, site.clone());

            SitePrices::<T>::remove(site_id);

            // Emit event
            Self::deposit_event(Event::SiteCreated(sender, site_id, site));

            // Return success
            Ok(())
        }

        #[pallet::weight(1000)]
        /// Burn a site
        pub fn burn(origin: OriginFor<T>, site_id: T::SiteIndex) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            Sites::<T>::try_mutate_exists(sender.clone(), site_id, |site| -> DispatchResult {
                // Test the user owns this site
                let _n = site.take().ok_or(Error::<T>::InvalidSiteId)?;
                let s = sender.clone();
                // Remove site from Sites data structure
                Sites::<T>::remove(sender, site_id);
                // Emit event
                Self::deposit_event(Event::SiteBurned(s, site_id));
                // Return success
                Ok(())
            })
        }

        #[pallet::weight(1000)]
        #[transactional]
        /// Burn a site
        pub fn modify(
            origin: OriginFor<T>,
            ipfs_cid: Vec<u8>,
            _site_name: Vec<u8>,
            site_id: T::SiteIndex,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let owner = sender.clone();

            Sites::<T>::try_mutate_exists(sender.clone(), site_id, |site| -> DispatchResult {
                // Test the user owns this site
                //let mut site_mod = site.take().ok_or(Error::<T>::InvalidSiteId)?;
                let mut site_mod = site.as_mut().ok_or(Error::<T>::InvalidSiteId)?;

                site_mod.ipfs_cid = ipfs_cid.clone();

                Self::deposit_event(Event::SiteCreated(owner, site_id, site_mod.clone()));

                Ok(())
            })?;
            Ok(())
        }

        /// Create a listing by setting a price for a site
        /// None to delist the site from SitePrices
        #[pallet::weight(1000)]
        pub fn listing(
            origin: OriginFor<T>,
            site_id: T::SiteIndex,
            new_price: Option<T::Balance>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            ensure!(
                <Sites<T>>::contains_key(&sender, site_id),
                Error::<T>::NotOwner
            );

            SitePrices::<T>::mutate_exists(site_id, |price| *price = new_price);
            Self::deposit_event(Event::SitePriceUpdated(sender, site_id, new_price));

            Ok(())
        }

        /// Buy a site
        #[pallet::weight(1000)]
        #[transactional]
        pub fn buy(
            origin: OriginFor<T>,
            owner: T::AccountId,
            site_id: T::SiteIndex,
            max_price: T::Balance,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            ensure!(sender != owner, Error::<T>::BuyFromSelf);

            Sites::<T>::try_mutate_exists(owner.clone(), site_id, |site| -> DispatchResult {
                let site = site.take().ok_or(Error::<T>::InvalidSiteId)?;

                SitePrices::<T>::try_mutate_exists(site_id, |price| -> DispatchResult {
                    let price = price.take().ok_or(Error::<T>::NotForSale)?;

                    ensure!(max_price >= price, Error::<T>::PriceTooLow);

                    <pallet_balances::Pallet<T> as Currency<T::AccountId>>::transfer(
                        &sender,
                        &owner,
                        price,
                        ExistenceRequirement::KeepAlive,
                    )?;

                    Sites::<T>::insert(&sender, site_id, site);

                    Self::deposit_event(Event::SiteSold(owner, sender, site_id, price));

                    Ok(())
                })
            })
        }

        #[pallet::weight(1000)]
        /// Transfer site to new owner
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            site_id: T::SiteIndex,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Sites::<T>::try_mutate_exists(sender.clone(), site_id, |site| -> DispatchResult {
                if sender == to {
                    ensure!(site.is_some(), Error::<T>::InvalidSiteId);
                    return Ok(());
                }
                let site = site.take().ok_or(Error::<T>::InvalidSiteId)?;
                Sites::<T>::insert(&to, site_id, site);
                Self::deposit_event(Event::SiteTransferred(sender, to, site_id));
                Ok(())
            })
        }
    }
}

impl<T: Config> Pallet<T> {
    fn get_next_note_id() -> Result<T::SiteIndex, DispatchError> {
        NextSiteId::<T>::try_mutate(|next_id| -> Result<T::SiteIndex, DispatchError> {
            let current_id = *next_id;
            *next_id = next_id
                .checked_add(&One::one())
                .ok_or(ArithmeticError::Overflow)?;
            Ok(current_id)
        })
    }
}
