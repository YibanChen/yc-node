use super::*;

use crate as site;
use frame_support::{assert_noop, assert_ok, parameter_types};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        SitesModule: site::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    // type AccountData = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = u64;
    type Event = Event;
    type DustRemoval = ();
    // type ExistentialDeposit = ExistentialDeposit;
    type ExistentialDeposit = ();
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

impl Config for Test {
    type Event = Event;
    type SiteIndex = u32;
}

// Build genesis storage according to the mock runtime.
// pub fn new_test_ext() -> sp_io::TestExternalities {
//     let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::default()
//         .build_storage::<Test>()
//         .unwrap()
//         .into();
//     t.execute_with(|| System::set_block_number(1));
//     t
// }

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(200, 500)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut t: sp_io::TestExternalities = t.into();

    t.execute_with(|| System::set_block_number(1));
    t
}

#[test]
fn can_create() {
    new_test_ext().execute_with(|| {
        assert_ok!(SitesModule::create(
            Origin::signed(100),
            "test".as_bytes().to_vec(),
            "test".as_bytes().to_vec()
        ));

        let site = Site {
            ipfs_cid: "test".as_bytes().to_vec(),
            site_name: "test".as_bytes().to_vec(),
        };

        assert_eq!(SitesModule::sites(100, 0), Some(site.clone()));
        assert_eq!(SitesModule::next_site_id(), 1);

        System::assert_last_event(Event::SitesModule(crate::Event::<Test>::SiteCreated(
            100, 0, site,
        )));
    });
}

#[test]
fn can_transfer() {
    new_test_ext().execute_with(|| {
        assert_ok!(SitesModule::create(
            Origin::signed(100),
            "test".as_bytes().to_vec(),
            "test".as_bytes().to_vec()
        ));

        assert_ok!(SitesModule::transfer(Origin::signed(100), 200, 0));

        System::assert_last_event(Event::SitesModule(crate::Event::SiteTransferred(
            100, 200, 0,
        )));
    });
}

#[test]
fn can_set_listing() {
    new_test_ext().execute_with(|| {
        assert_ok!(SitesModule::create(
            Origin::signed(100),
            "test".as_bytes().to_vec(),
            "test".as_bytes().to_vec()
        ));

        assert_noop!(
            SitesModule::listing(Origin::signed(200), 0, Some(10)),
            Error::<Test>::NotOwner
        );

        assert_ok!(SitesModule::listing(Origin::signed(100), 0, Some(10)));

        System::assert_last_event(Event::SitesModule(crate::Event::SitePriceUpdated(
            100,
            0,
            Some(10),
        )));

        assert_eq!(SitesModule::site_prices(0), Some(10));

        assert_ok!(SitesModule::listing(Origin::signed(100), 0, None));
        assert_eq!(SitePrices::<Test>::contains_key(0), false);

        System::assert_last_event(Event::SitesModule(crate::Event::SitePriceUpdated(
            100, 0, None,
        )));
    });
}

#[test]
fn can_buy() {
    new_test_ext().execute_with(|| {
        assert_ok!(SitesModule::create(
            Origin::signed(100),
            "test".as_bytes().to_vec(),
            "test".as_bytes().to_vec()
        ));

        let site = SitesModule::sites(100, 0).unwrap();

        assert_noop!(
            SitesModule::buy(Origin::signed(100), 100, 0, 10),
            Error::<Test>::BuyFromSelf
        );
        assert_noop!(
            SitesModule::buy(Origin::signed(200), 100, 1, 10),
            Error::<Test>::InvalidSiteId
        );
        assert_noop!(
            SitesModule::buy(Origin::signed(200), 100, 0, 10),
            Error::<Test>::NotForSale
        );

        assert_ok!(SitesModule::listing(Origin::signed(100), 0, Some(400)));

        assert_ok!(SitesModule::buy(Origin::signed(200), 100, 0, 500));

        assert_eq!(SitePrices::<Test>::contains_key(0), false);
        assert_eq!(Sites::<Test>::contains_key(100, 0), false);
        assert_eq!(SitesModule::sites(200, 0), Some(site));
        assert_eq!(Balances::free_balance(100), 400);
        assert_eq!(Balances::free_balance(200), 100);

        System::assert_last_event(Event::SitesModule(crate::Event::SiteSold(100, 200, 0, 400)));
    });
}
