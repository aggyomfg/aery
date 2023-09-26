use crate::tuple_traits::*;

use bevy::ecs::{
    entity::Entity,
    query::{ReadOnlyWorldQuery, WorldQuery},
    system::Query,
};

use std::{borrow::Borrow, collections::VecDeque};

use crate::{
    for_each::ControlFlow,
    operations::utils::{EdgeSide, Operations, Relations, RelationsItem},
};

/// A 3 arity version of [`ForEachPermutations`] for when operations feature a traversal with 1 or
/// more joins. Will iterate through hierarchy permutations and join permutations together.
/// - The left and middle paramaters will be an ancestor/descendant pairs.
/// - The rightmost parameter will be a tuple of all the query fetch parameters from joined queries
/// where the entity being joined on is the same entity that is the left parameter. This is the
/// ancestor or descendant depending on if the traversal is an ascent or descent.
pub trait ForEachPermutations3Arity<const N: usize> {
    type P0<'p0>;
    type P1<'p1>;
    type P2<'p2>;

    fn for_each<Func, Ret>(self, func: Func)
    where
        Ret: Into<ControlFlow>,
        Func: for<'f, 'p0, 'p1, 'p2> FnMut(
            &'f mut Self::P0<'p0>,
            &'f mut Self::P1<'p1>,
            Self::P2<'p2>,
        ) -> Ret;
}

impl<Q, RS, F, T, JoinedTypes, JoinedQueries, const N: usize> ForEachPermutations3Arity<N>
    for Operations<&'_ Query<'_, '_, (Q, Relations<RS>), F>, JoinedTypes, JoinedQueries, T, Entity>
where
    Q: WorldQuery,
    RS: RelationSet,
    F: ReadOnlyWorldQuery,
    T: EdgeSide,
    JoinedTypes: Product<N>,
    JoinedQueries: for<'a> Joinable<'a, N>,
    for<'i> RelationsItem<'i, RS>: RelationEntries,
{
    type P0<'p0> = <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'p0>;
    type P1<'p1> = <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'p1>;
    type P2<'p2> = <JoinedQueries as Joinable<'p2, N>>::Out;

    fn for_each<Func, Ret>(mut self, mut func: Func)
    where
        Ret: Into<ControlFlow>,
        Func: for<'f, 'p0, 'p1, 'p2> FnMut(
            &'f mut Self::P0<'p0>,
            &'f mut Self::P1<'p1>,
            Self::P2<'p2>,
        ) -> Ret,
    {
        let mut queue = VecDeque::from([self.start]);

        'queue: while let Some(entity) = queue.pop_front() {
            let Ok((mut left_components, left_edges)) = self.control.get(entity) else {
                continue;
            };

            for mid in T::entities(&left_edges) {
                let Ok((mut mid_components, mid_edges)) = self.control.get(mid) else {
                    continue;
                };

                let mut edge_product = JoinedTypes::product(&mid_edges);
                let mut matches = [false; N];

                while let Some(entities) = edge_product.advance(matches) {
                    matches = Joinable::check(&self.joined_queries, entities);

                    if matches.iter().any(|b| !b) {
                        continue;
                    }

                    match func(
                        &mut left_components,
                        &mut mid_components,
                        Joinable::join(&mut self.joined_queries, entities),
                    )
                    .into()
                    {
                        ControlFlow::Continue => {}
                        ControlFlow::Exit => return,
                        ControlFlow::Walk => break,
                        ControlFlow::FastForward(n) if n < N => {
                            matches[n] = false;
                        }
                        ControlFlow::Conclude => {
                            continue 'queue;
                        }
                        ControlFlow::Probe => {
                            queue.clear();
                            queue.push_back(mid);
                            continue 'queue;
                        }
                        _ => {}
                    }
                }
            }

            queue.extend(T::entities(&left_edges));
        }
    }
}

impl<Q, RS, F, T, JoinedTypes, JoinedQueries, const N: usize> ForEachPermutations3Arity<N>
    for Operations<
        &'_ mut Query<'_, '_, (Q, Relations<RS>), F>,
        JoinedTypes,
        JoinedQueries,
        T,
        Entity,
    >
where
    Q: WorldQuery,
    RS: RelationSet,
    F: ReadOnlyWorldQuery,
    T: EdgeSide,
    JoinedTypes: Product<N>,
    JoinedQueries: for<'a> Joinable<'a, N>,
    for<'i> RelationsItem<'i, RS>: RelationEntries,
{
    type P0<'p0> = <Q as WorldQuery>::Item<'p0>;
    type P1<'p1> = <Q as WorldQuery>::Item<'p1>;
    type P2<'p2> = <JoinedQueries as Joinable<'p2, N>>::Out;

    fn for_each<Func, Ret>(mut self, mut func: Func)
    where
        Ret: Into<ControlFlow>,
        Func: for<'f, 'p0, 'p1, 'p2> FnMut(
            &'f mut Self::P0<'p0>,
            &'f mut Self::P1<'p1>,
            Self::P2<'p2>,
        ) -> Ret,
    {
        let mut queue = VecDeque::from([self.start]);

        'queue: while let Some(entity) = queue.pop_front() {
            // SAFETY: Self referential relations are impossible so this is always safe.
            let Ok((mut left_components, left_edges)) =
                (unsafe { self.control.get_unchecked(entity) })
            else {
                continue;
            };

            for mid in T::entities(&left_edges) {
                // SAFETY: Self referential relations are impossible so this is always safe.
                let Ok((mut mid_components, mid_edges)) =
                    (unsafe { self.control.get_unchecked(mid) })
                else {
                    continue;
                };

                let mut edge_product = JoinedTypes::product(&mid_edges);
                let mut matches = [false; N];

                while let Some(entities) = edge_product.advance(matches) {
                    matches = Joinable::check(&self.joined_queries, entities);

                    if matches.iter().any(|b| !b) {
                        continue;
                    }

                    match func(
                        &mut left_components,
                        &mut mid_components,
                        Joinable::join(&mut self.joined_queries, entities),
                    )
                    .into()
                    {
                        ControlFlow::Continue => {}
                        ControlFlow::Exit => return,
                        ControlFlow::Walk => break,
                        ControlFlow::FastForward(n) if n < N => {
                            matches[n] = false;
                        }
                        ControlFlow::Conclude => {
                            continue 'queue;
                        }
                        ControlFlow::Probe => {
                            queue.clear();
                            queue.push_back(mid);
                            continue 'queue;
                        }
                        _ => {}
                    }
                }
            }

            queue.extend(T::entities(&left_edges));
        }
    }
}

impl<Q, RS, F, T, Acc, Err, Init, Fold> ForEachPermutations3Arity<0>
    for Operations<&'_ Query<'_, '_, (Q, Relations<RS>), F>, (), (), T, Entity, (), (), Init, Fold>
where
    Q: WorldQuery,
    RS: RelationSet,
    F: ReadOnlyWorldQuery,
    T: EdgeSide,
    for<'i> RelationsItem<'i, RS>: RelationEntries,
    Init: for<'a> FnMut(&mut <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'a>) -> Acc,
    Fold: for<'a> FnMut(
        Acc,
        <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'a>,
    ) -> Result<Acc, Err>,
{
    type P0<'p0> = <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'p0>;
    type P1<'p1> = Result<Acc, Err>;
    type P2<'p2> = <<Q as WorldQuery>::ReadOnly as WorldQuery>::Item<'p2>;

    fn for_each<Func, Ret>(mut self, mut func: Func)
    where
        Ret: Into<ControlFlow>,
        Func: for<'f, 'p0, 'p1, 'p2> FnMut(
            &'f mut Self::P0<'p0>,
            &'f mut Self::P1<'p1>,
            Self::P2<'p2>,
        ) -> Ret,
    {
        let mut queue = VecDeque::from([self.start]);

        'queue: while let Some(entity) = queue.pop_front() {
            let Ok((mut control, relations)) = self.control.get(entity) else {
                continue;
            };

            let mut acc = Ok::<_, Err>((self.init)(&mut control));

            for e in T::entities(&relations) {
                let Ok(traversal_item) = self.control.get(e) else {
                    continue;
                };

                let Ok(acc_ok) = acc else {
                    break;
                };

                acc = (self.fold)(acc_ok, traversal_item.0);
            }

            for e in T::entities(&relations) {
                let Ok(traversal_item) = self.control.get(e) else {
                    continue;
                };

                match func(&mut control, &mut acc, traversal_item.0).into() {
                    ControlFlow::Exit => return,
                    ControlFlow::Conclude => {
                        continue 'queue;
                    }
                    ControlFlow::Probe => {
                        queue.clear();
                        queue.push_back(e);
                        continue 'queue;
                    }
                    _ => {}
                }
            }

            queue.extend(T::entities(&relations));
        }
    }
}

impl<Q, RS, F, T, Acc, Err, Init, Fold> ForEachPermutations3Arity<0>
    for Operations<
        &'_ mut Query<'_, '_, (Q, Relations<RS>), F>,
        (),
        (),
        T,
        Entity,
        (),
        (),
        Init,
        Fold,
    >
where
    Q: WorldQuery,
    RS: RelationSet,
    F: ReadOnlyWorldQuery,
    T: EdgeSide,
    for<'i> RelationsItem<'i, RS>: RelationEntries,
    Init: for<'a> FnMut(&mut <Q as WorldQuery>::Item<'a>) -> Acc,
    Fold: for<'a> FnMut(Acc, <Q as WorldQuery>::Item<'a>) -> Result<Acc, Err>,
{
    type P0<'p0> = <Q as WorldQuery>::Item<'p0>;
    type P1<'p1> = Result<Acc, Err>;
    type P2<'p2> = <Q as WorldQuery>::Item<'p2>;

    fn for_each<Func, Ret>(mut self, mut func: Func)
    where
        Ret: Into<ControlFlow>,
        Func: for<'f, 'p0, 'p1, 'p2> FnMut(
            &'f mut Self::P0<'p0>,
            &'f mut Self::P1<'p1>,
            Self::P2<'p2>,
        ) -> Ret,
    {
        let mut queue = VecDeque::from([self.start]);

        'queue: while let Some(entity) = queue.pop_front() {
            // SAFETY: Self referential relations are impossible so this is always safe.
            let Ok((mut control, relations)) = (unsafe { self.control.get_unchecked(entity) })
            else {
                continue;
            };

            let mut acc = Ok::<_, Err>((self.init)(&mut control));

            for e in T::entities(&relations) {
                // SAFETY: Self referential relations are impossible so this is always safe.
                let Ok(traversal_item) = (unsafe { self.control.get_unchecked(e) }) else {
                    continue;
                };

                let Ok(acc_ok) = acc else {
                    break;
                };

                acc = (self.fold)(acc_ok, traversal_item.0);
            }

            for e in T::entities(&relations) {
                // SAFETY: Self referential relations are impossible so this is always safe.
                let Ok(traversal_item) = (unsafe { self.control.get_unchecked(e) }) else {
                    continue;
                };

                match func(&mut control, &mut acc, traversal_item.0).into() {
                    ControlFlow::Exit => return,
                    ControlFlow::Conclude => {
                        continue 'queue;
                    }
                    ControlFlow::Probe => {
                        queue.clear();
                        queue.push_back(e);
                        continue 'queue;
                    }
                    _ => {}
                }
            }

            queue.extend(T::entities(&relations));
        }
    }
}
