//! #PESA pallet for Substrate.
//!
//! The PESA pallet for Substrate allows account holders to register
//! a phone number as an account alias, while providing lookup and
//! reverse lookup convenience extrinsic functions.
//!
//! Helper functions are also provided to transfer, update and
//! remove aliases.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::{
    decl_module, decl_storage, decl_event, decl_error, ensure, RuntimeDebug,
    dispatch::{DispatchResult},
    traits::{Currency, Get},
};

use sp_std::vec::Vec;
use frame_system::ensure_signed;

// type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
// type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
type UserInfoOf<T> = UserInfo<<T as frame_system::Config>::AccountId>;

pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    type Currency: Currency<Self::AccountId>;
    type Transfer: Get<bool>;
    /// Set in runtime configuration. The max (inclusive) number of u8 characters allowed
    /// to be set to a phone number alias
	type NumberMaxLength: Get<u32>;
	/// Set in runtime configuration. The min (inclusive) number of u8 characters allowed
	/// to be set to a phone number alias
	type NumberMinLength: Get<u32>;
}

/// Custom struct type to hold user data within the substrate storage maps.
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq, Default)]
pub struct UserInfo<AccountId>
{
    /// owner account
	pub owner: AccountId,
    /// available for public look up
	pub public: bool,
	/// a new number can be transferred to this account
    pub transferable: bool,
	/// phone number alias to store
    pub phone: Phone,
}

/// Custom struct type to hold phone number aliases
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq, Default)]
pub struct Phone(Vec<u8>);

decl_storage! {
	trait Store for Module<T: Config> as Pesa {
		/// Storage map to look up a phone number alias by account ID
		pub PhoneLookUp get(fn phone_look_up): map hasher(blake2_128_concat) T::AccountId => Option<UserInfoOf<T>>;
		/// Storage map to look up account ID by phone number alias
		pub AccountLookUp get(fn account_look_up): map hasher(blake2_128_concat) Phone => Option<UserInfoOf<T>>;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Config>::AccountId,
	{
		/// Phone alias transferred to second account
		NumberTransfered(AccountId, AccountId),
		/// Phone alias removed
		NumberRemoved(),
		/// Phone alias lookup success \[phone\]
		LookUpSuccess(Vec<u8>),
		/// Account found \[accountID\]
		AccountFound(AccountId),
		/// Account alias successfully registered
		SuccessfullRegsitration(),
		/// Account data returned \[accountID\] \[is_public\] \[is_transferable_to\] \[phone\]
		AccountData(AccountId, bool, bool, Vec<u8>),
		/// Allowed alias to transfer to this account,
		TransferableSet(),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// Number overflow occurred
		NumberOverflow,
		/// Invalid phone alias transfer
		InvalidTransfer,
		/// Phone alias not enough characters
		NumberTooShort,
		/// Phone alias too many characters
		NumberTooLong,
		/// Phone alias already exists. No duplicates allowed
		NumberAlreadyExists,
		/// Look up failure. Cannot return results
		LookupFailure,
		/// Look up alias is not public
		NumberNotPublic,
		/// Incorrect data submitted
		IncorrectInformation,
		/// Number does not exist in the system
		NumberDoesNotExist,
		/// Account does not exist in the system
		AccountDoesNotExist,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		/// Register a phone number alias to the current Substrate account. Excessively long or short input will result in an error.
		/// PhoneNumber / Number Public for Lookup / Is Public
		#[weight = 1000]
		pub fn register(origin, phone_number: Phone, public: bool, new_account: bool) -> DispatchResult
		{
			let account = ensure_signed(origin)?;
			let phone_size = get_usize_safe(phone_number.0.len()).ok_or_else(|| Error::<T>::NumberOverflow)?;

			ensure!(phone_size >= T::NumberMinLength::get(), Error::<T>::NumberTooShort);
			ensure!(phone_size <= T::NumberMaxLength::get(), Error::<T>::NumberTooLong);

		//	new accounts should not already be in storage

			if new_account{
				ensure!(!<PhoneLookUp<T>>::contains_key(&account), Error::<T>::NumberAlreadyExists);
				ensure!(!<AccountLookUp<T>>::contains_key(&phone_number), Error::<T>::NumberAlreadyExists);
			}

			// existing accounts should already be in storage
			if !new_account{
				ensure!(<PhoneLookUp<T>>::contains_key(&account), Error::<T>::NumberDoesNotExist);
				ensure!(<AccountLookUp<T>>::contains_key(&phone_number), Error::<T>::NumberDoesNotExist);
			}

			let user_info = UserInfo
			{
				owner: account.clone(),
				public: public,
				transferable: false,
				phone: phone_number.clone(),
			};

			<AccountLookUp<T>>::insert(phone_number, &user_info);
			<PhoneLookUp<T>>::insert(account, user_info);

			Self::deposit_event(RawEvent::SuccessfullRegsitration());
			Ok(())
		}


		/// Look up an account ID associated with a phone number alias
		#[weight = 1000]
		pub fn look_up(origin, phone_number: Phone) -> DispatchResult
		{
			let _account = ensure_signed(origin)?;

			// try look up number and check if public listed
			let user = Self::account_look_up(phone_number).ok_or(Error::<T>::LookupFailure)?;
			ensure!(user.public, Error::<T>::NumberNotPublic);

			// return user account
			Self::deposit_event(RawEvent::AccountFound(user.owner));
			Ok(())

		}

		/// Look up a phone number alias associate with an account ID
		#[weight = 1000]
		pub fn reverse_look_up(origin, check_account: T::AccountId) -> DispatchResult
		{
			let _account = ensure_signed(origin)?;

			let user = Self::phone_look_up(check_account).ok_or(Error::<T>::LookupFailure)?;
			ensure!(user.public, Error::<T>::NumberNotPublic);

			Self::deposit_event(RawEvent::LookUpSuccess(user.phone.0));
			Ok(())

		}

		/// Fetch own account data
		#[weight = 1000]
		pub fn account_data(origin) -> DispatchResult
		{
			let account = ensure_signed(origin)?;
			let user = Self::phone_look_up(&account).ok_or(Error::<T>::LookupFailure)?;

			// Account ID, public, transferable, phone
			Self::deposit_event(RawEvent::AccountData(account, user.public, user.transferable, user.phone.0));
			Ok(())

		}

		///Enable permissions for a one time transfer of a phone number alias to an origin account
		#[weight = 1000]
		pub fn allow_tranfer(origin) -> DispatchResult
		{
			let account = ensure_signed(origin)?;

			ensure!(<PhoneLookUp<T>>::contains_key(&account), Error::<T>::AccountDoesNotExist);
			let mut user = Self::phone_look_up(&account).ok_or(Error::<T>::LookupFailure)?;
			ensure!(<AccountLookUp<T>>::contains_key(&user.phone), Error::<T>::NumberDoesNotExist);

			user.transferable = true;

			PhoneLookUp::<T>::insert(account, &user);
			AccountLookUp::<T>::insert(&user.phone, &user);

			Self::deposit_event(RawEvent::TransferableSet());

			Ok(())

			// Dev note: Tried using mutate exists, but seems better practice to check first if
			// exists in both databases and then fail fast, rather than mutate and change in one,
			// but possibly fail in the second

			// PhoneLookUp::<T>::try_mutate_exists(account.clone(), |user_info| -> DispatchResult
			// {
			// 		let mut user_info = user_info.take().ok_or(Error::<T>::IncorrectInformation)?;
			// 		user_info.transferable = true;
			// 		PhoneLookUp::<T>::insert(&account, user_info);
			// 		Ok(())
			// })?
		}

		/// Transfer phone number alias to a second account. The second account (transfer_account) must have
		/// allow transferred enabled first. Once the transfer has occurred, the allow transfer toggle will
		/// be disabled again
		#[weight = 1000]
		pub fn tranfer(origin, tranfer_account: T::AccountId) -> DispatchResult
		{
			let _account = ensure_signed(origin)?;

			//todo 1
			//get ops phone number
			// get transer accounts user info
			// check that num can be transfered to
			// add numbers to new account
			// remove numbers from old account

			Ok(())
		}

		/// Clear number alias data from storage.
		#[weight = 1000]
		pub fn clear_data(origin) -> DispatchResult
		{
			let account = ensure_signed(origin)?;

			ensure!(<PhoneLookUp<T>>::contains_key(&account), Error::<T>::IncorrectInformation);
			let user = Self::phone_look_up(&account).ok_or(Error::<T>::LookupFailure)?;
			ensure!(<AccountLookUp<T>>::contains_key(&user.phone), Error::<T>::IncorrectInformation);

			<AccountLookUp<T>>::remove(&user.phone);
			<PhoneLookUp<T>>::remove(account);

			Self::deposit_event(RawEvent::NumberRemoved());
			Ok(())
		}
	}
}

/// Covert uSize to u32 without using the standard library
fn get_usize_safe(v: usize) -> Option<u32> {
    if v > u32::MAX as usize {
        None
    } else {
        Some(v as u32)
    }
}
