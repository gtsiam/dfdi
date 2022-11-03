use std::{
    any::{type_name, TypeId},
    collections::HashMap,
    marker::PhantomData,
    ptr::NonNull,
};

use crate::{BindError, ProvideFn, Provider, Service, UnbindError};

/// A context in which to store providers for services
pub struct Context<'pcx> {
    /// Map `Service` `TypeId`s to a type-erased provider
    //
    // Note: Unfortunately, https://github.com/rust-lang/rust/issues/10389 is an I-unsound bug to
    // keep an eye on. TL;DR: TypeId hash collisions are possible and there have been some (obscure)
    // examples of this in the past.
    providers: HashMap<TypeId, DynProvider>,

    _phantom: PhantomData<&'pcx ()>,
}

impl Context<'_> {
    /// Create an empty context
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Create a sub-context
    ///
    /// The retuned context will contain the same elements as the parent context and any elements
    /// added to the sub context will not be visible on the original
    pub fn scoped(&self) -> Context<'_> {
        // Notes:
        // - We are cloning the pointers, not the underlying data
        // - Provider expects a shared reference
        // - DynProvider's clone implementation skips the drop function for clones
        Context {
            providers: self.providers.clone(),
            _phantom: PhantomData,
        }
    }

    /// Register a new provider for the service `S`
    #[track_caller]
    pub fn bind_with<'cx, S: Service>(&'cx mut self, provider: impl Provider<'cx, S>) {
        if let Err(err) = self.try_bind_with(provider) {
            panic!("{}", err)
        }
    }

    #[track_caller]
    pub fn bind<'cx, S, P>(&'cx mut self)
    where
        S: Service,
        P: Provider<'cx, S> + Default,
    {
        if let Err(err) = self.try_bind::<S, P>() {
            panic!("{}", err)
        }
    }

    /// Delete the provider bound to the specified service `S`
    pub fn unbind<S>(&mut self)
    where
        S: Service,
    {
        if let Err(err) = self.try_unbind::<S>() {
            panic!("{}", err)
        }
    }

    /// Resolve the service `S` based on the already registered providers
    ///
    /// # Panics
    /// If no provider is registered for this service
    #[track_caller]
    pub fn resolve<S: Service>(&self) -> S::Output<'_> {
        match self.try_resolve::<S>() {
            Some(s) => s,
            None => panic!("no provider for service `{}`", type_name::<S>()),
        }
    }

    /// Register a new provider for the service `S`
    pub fn try_bind_with<'cx, S: Service>(
        &'cx mut self,
        provider: impl Provider<'cx, S>,
    ) -> Result<(), BindError> {
        use std::collections::hash_map::Entry::*;
        match self.providers.entry(TypeId::of::<S>()) {
            Vacant(e) => {
                // SAFETY:
                // - Due to the api provided by `Context`, all clones of `DynProvider` _will_ be
                //   dropped before the original instance is dropped
                e.insert(unsafe { DynProvider::new(provider) });
                Ok(())
            }
            Occupied(_) => Err(BindError::ServiceBound(std::any::type_name::<S>())),
        }
    }

    #[inline(always)]
    pub fn try_bind_fn<'cx, S: Service>(
        &'cx mut self,
        provider_fn: impl Fn(&'cx Context) -> S::Output<'cx> + 'cx,
    ) -> Result<(), BindError> {
        self.try_bind_with::<S>(provider_fn)
    }

    /// Register a new provider for the service `S`, built from the provider's
    /// [`std::default::Default`] implementation
    #[inline(always)]
    pub fn try_bind<'cx, S, P>(&'cx mut self) -> Result<(), BindError>
    where
        S: Service,
        P: Provider<'cx, S> + Default,
    {
        self.try_bind_with(P::default())
    }

    /// Delete the provider bound to the specified service `S`
    pub fn try_unbind<S>(&mut self) -> Result<(), UnbindError>
    where
        S: Service,
    {
        match self.providers.remove(&TypeId::of::<S>()) {
            Some(_) => Ok(()),
            None => Err(UnbindError::ServiceUnbound(type_name::<S>())),
        }
    }

    /// Try to resolve a component based on the already registered providers
    ///
    /// Returns `None` if no provider is registered for this [`Service`].
    pub fn try_resolve<S: Service>(&self) -> Option<S::Output<'_>> {
        let provider = self.providers.get(&TypeId::of::<S>())?;

        // SAFETY:
        // - We know that the provider was created for the service `S`, since it came from the
        //   `self.providers` map
        Some(unsafe { provider.provide::<S>(self) })
    }
}

impl Default for Context<'_> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

struct DynProvider {
    /// Type-erased pointer to the underlying provider data
    this: NonNull<()>,

    /// Type-erased function pointer to the provider's `provide` implementation
    provide_fn: NonNull<()>,

    /// Pointer to the provider's `drop` implementation
    //
    // SAFETY:
    // - Must only be called with a valid `self.this` pointer
    drop_fn: Option<unsafe fn(*mut ())>,
}

impl DynProvider {
    /// Create a `DynProvider` for the service `S`
    ///
    /// SAFETY:
    /// - This instance must live as long as all of its clones
    unsafe fn new<'cx, S, P>(provider: P) -> Self
    where
        S: Service,
        P: Provider<'cx, S>,
    {
        unsafe fn drop_provider<P>(this: *mut ()) {
            std::mem::drop(Box::from_raw(this as *mut P));
        }

        // Create a pointer to a specialized `drop` function and store it.
        let drop_fn = Some(drop_provider::<P> as _);

        // Get the P::provide function pointer and store a type-erased version of it
        //
        // SAFETY:
        // - fn pointers are always non-null
        let provide_fn = unsafe { NonNull::new_unchecked(P::provide as fn(_, _) -> _ as _) };

        // Create the `this` pointer.
        //
        // SAFETY:
        // - A `Box`'s pointer is always guaranteed to be non-null
        let this = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(provider)) as *mut _) };

        Self {
            this,
            drop_fn,
            provide_fn,
        }
    }

    /// Run the provider
    ///
    /// SAFETY:
    /// - The `DynProvider` was created for the service `S`
    unsafe fn provide<'cx, S>(&'cx self, cx: &'cx Context) -> S::Output<'cx>
    where
        S: Service,
    {
        let this = self.this.as_ptr() as *const ();
        let provide_fn: ProvideFn<'cx, S> = std::mem::transmute(self.provide_fn);

        provide_fn(this, cx)
    }
}

impl Clone for DynProvider {
    fn clone(&self) -> Self {
        Self {
            this: self.this,
            provide_fn: self.provide_fn,
            drop_fn: None, // drop should only run on the original instance
        }
    }
}

impl Drop for DynProvider {
    fn drop(&mut self) {
        if let Some(drop_fn) = self.drop_fn {
            // SAFETY:
            // - `drop_fn` can only be called with `self.this`, which it is.
            // - We know drop has not been called because of the safety guarantees on new(), which
            //   means that `self.this` points to valid memory.
            unsafe { (drop_fn)(self.this.as_ptr()) }
        }
    }
}
