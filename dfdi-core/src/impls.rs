use std::{rc::Rc, sync::Arc};

use crate::{Context, Provider, Service};

/// Allow `Fn` functions to act as providers.
impl<'cx, F, S> Provider<'cx, S> for F
where
    F: Fn(&'cx Context) -> S::Output<'cx> + 'cx,
    S: Service,
{
    fn provide(&'cx self, cx: &'cx Context) -> S::Output<'cx> {
        (self)(cx)
    }
}

impl<'cx, S, P> Provider<'cx, Option<S>> for Option<P>
where
    S: Service,
    P: Provider<'cx, S>,
{
    fn provide(&'cx self, cx: &'cx Context) -> Option<S::Output<'cx>> {
        self.as_ref().map(|p| p.provide(cx))
    }
}

// Generic common types

impl<const N: usize, S: Service> Service for [S; N] {
    type Output<'cx> = [S::Output<'cx>; N];
}

macro_rules! service_impl_generic {
    ($($ty:ty => $out:ty,)*) => {
        $( impl <S: Service> Service for $ty { type Output<'cx> = $out; } )*
    };
}

service_impl_generic! {
    &'static S => &'cx S::Output<'cx>,
    &'static mut S => &'cx mut S::Output<'cx>,
    Option<S> => Option<S::Output<'cx>>,
    Box<S> => Box<S::Output<'cx>>,
    Rc<S> => Rc<S::Output<'cx>>,
    Arc<S> => Arc<S::Output<'cx>>,
}

macro_rules! service_impl_tuples {
    ($( ( $($param:ident),* ), )*) => {
        $(
            impl<$($param),*> Service for ($($param,)*)
            where
                $($param: Service,)*
            {
                type Output<'cx> = ($($param::Output<'cx>,)*);
            }
        )*
    };
}

service_impl_tuples! {
    (S1),
    (S1, S2),
    (S1, S2, S3),
    (S1, S2, S3, S4),
    (S1, S2, S3, S4, S5),
    (S1, S2, S3, S4, S5, S6),
    (S1, S2, S3, S4, S5, S6, S7),
    (S1, S2, S3, S4, S5, S6, S7, S8),
    (S1, S2, S3, S4, S5, S6, S7, S8, S9),
}
