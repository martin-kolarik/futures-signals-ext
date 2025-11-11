use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};

use futures_signals::signal_vec::{SignalVec, SignalVecExt, VecDiff};
use pin_project_lite::pin_project;

pub(crate) struct FlattenState<A> {
    signal_vec: Option<Pin<Box<A>>>,
    len: usize,
}

impl<A> FlattenState<A>
where
    A: SignalVec,
{
    fn new(signal_vec: A) -> Self {
        Self {
            signal_vec: Some(Box::pin(signal_vec)),
            len: 0,
        }
    }

    fn update_len(&mut self, diff: &VecDiff<A::Item>) {
        match diff {
            VecDiff::Replace { values } => {
                self.len = values.len();
            }
            VecDiff::InsertAt { .. } | VecDiff::Push { .. } => {
                self.len += 1;
            }
            VecDiff::RemoveAt { .. } | VecDiff::Pop {} => {
                self.len -= 1;
            }
            VecDiff::Clear {} => {
                self.len = 0;
            }
            VecDiff::UpdateAt { .. } | VecDiff::Move { .. } => {}
        }
    }

    fn poll(&mut self, cx: &mut Context) -> Option<Poll<Option<VecDiff<A::Item>>>> {
        self.signal_vec
            .as_mut()
            .map(|s| s.poll_vec_change_unpin(cx))
    }

    fn poll_values(&mut self, cx: &mut Context) -> Vec<A::Item> {
        let mut output = vec![];

        loop {
            match self.poll(cx) {
                Some(Poll::Ready(Some(diff))) => {
                    self.update_len(&diff);
                    diff.apply_to_vec(&mut output);
                }
                Some(Poll::Ready(None)) => {
                    self.signal_vec = None;
                    break;
                }
                Some(Poll::Pending) | None => {
                    break;
                }
            }
        }

        output
    }

    fn poll_pending(
        &mut self,
        cx: &mut Context,
        prev_len: usize,
        pending: &mut PendingBuilder<VecDiff<A::Item>>,
    ) -> bool {
        loop {
            return match self.poll(cx) {
                Some(Poll::Ready(Some(diff))) => {
                    let old_len = self.len;

                    self.update_len(&diff);

                    match diff {
                        VecDiff::Replace { values } => {
                            for index in (0..old_len).rev() {
                                pending.push(VecDiff::RemoveAt {
                                    index: prev_len + index,
                                });
                            }

                            for (index, value) in values.into_iter().enumerate() {
                                pending.push(VecDiff::InsertAt {
                                    index: prev_len + index,
                                    value,
                                });
                            }
                        }
                        VecDiff::InsertAt { index, value } => {
                            pending.push(VecDiff::InsertAt {
                                index: prev_len + index,
                                value,
                            });
                        }
                        VecDiff::UpdateAt { index, value } => {
                            pending.push(VecDiff::UpdateAt {
                                index: prev_len + index,
                                value,
                            });
                        }
                        VecDiff::RemoveAt { index } => {
                            pending.push(VecDiff::RemoveAt {
                                index: prev_len + index,
                            });
                        }
                        VecDiff::Move {
                            old_index,
                            new_index,
                        } => {
                            pending.push(VecDiff::Move {
                                old_index: prev_len + old_index,
                                new_index: prev_len + new_index,
                            });
                        }
                        VecDiff::Push { value } => {
                            pending.push(VecDiff::InsertAt {
                                index: prev_len + old_len,
                                value,
                            });
                        }
                        VecDiff::Pop {} => {
                            pending.push(VecDiff::RemoveAt {
                                index: prev_len + (old_len - 1),
                            });
                        }
                        VecDiff::Clear {} => {
                            for index in (0..old_len).rev() {
                                pending.push(VecDiff::RemoveAt {
                                    index: prev_len + index,
                                });
                            }
                        }
                    }

                    continue;
                }
                Some(Poll::Ready(None)) => {
                    self.signal_vec = None;
                    true
                }
                Some(Poll::Pending) => false,
                None => true,
            };
        }
    }
}

pin_project! {
    #[must_use = "SignalVecs do nothing unless polled"]
    pub struct Flatten<A>
    where
        A: SignalVec,
        A::Item: SignalVec,
    {
        #[pin]
        pub(crate) signal: Option<A>,
        pub(crate) inner: Vec<FlattenState<A::Item>>,
        pub(crate) pending: VecDeque<VecDiff<<A::Item as SignalVec>::Item>>,
    }
}

fn fill_removals<A>(
    inner: &[FlattenState<A>],
    index: usize,
    pending: &mut PendingBuilder<VecDiff<A::Item>>,
) where
    A: SignalVec,
{
    let removed_len = inner[index].len;
    let prev_len: usize = inner[..index].iter().map(|state| state.len).sum();
    for index in (0..removed_len).rev() {
        pending.push(VecDiff::RemoveAt {
            index: prev_len + index,
        });
    }
}

fn fill_moves<A>(
    inner: &[FlattenState<A>],
    old_index: usize,
    new_index: usize,
    pending: &mut PendingBuilder<VecDiff<A::Item>>,
) where
    A: SignalVec,
{
    let moved_len = inner[old_index].len;
    let old_prev_len: usize = inner[..old_index].iter().map(|state| state.len).sum();
    let new_prev_len: usize = inner[..new_index].iter().map(|state| state.len).sum();
    if new_index < old_index {
        (0..moved_len).for_each(|_| {
            pending.push(VecDiff::Move {
                old_index: old_prev_len + moved_len - 1,
                new_index: new_prev_len,
            })
        });
    } else {
        (0..moved_len).for_each(|_| {
            pending.push(VecDiff::Move {
                old_index: old_prev_len,
                new_index: new_prev_len + moved_len - 1,
            })
        });
    }
}

impl<A> SignalVec for Flatten<A>
where
    A: SignalVec,
    A::Item: SignalVec,
{
    type Item = <A::Item as SignalVec>::Item;

    fn poll_vec_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<VecDiff<Self::Item>>> {
        let mut this = self.project();

        if let Some(diff) = this.pending.pop_front() {
            return Poll::Ready(Some(diff));
        }

        let mut pending: PendingBuilder<VecDiff<Self::Item>> = PendingBuilder::new();

        let top_done = loop {
            break match this
                .signal
                .as_mut()
                .as_pin_mut()
                .map(|signal| signal.poll_vec_change(cx))
            {
                Some(Poll::Ready(Some(diff))) => {
                    match diff {
                        VecDiff::Replace { values } => {
                            *this.inner = values.into_iter().map(FlattenState::new).collect();

                            let values = this
                                .inner
                                .iter_mut()
                                .flat_map(|state| state.poll_values(cx))
                                .collect();

                            return Poll::Ready(Some(VecDiff::Replace { values }));
                        }
                        VecDiff::InsertAt { index, value } => {
                            this.inner.insert(index, FlattenState::new(value));
                        }
                        VecDiff::UpdateAt { index, value } => {
                            fill_removals(&this.inner, index, &mut pending);
                            this.inner[index] = FlattenState::new(value);
                        }
                        VecDiff::RemoveAt { index } => {
                            fill_removals(&this.inner, index, &mut pending);
                            this.inner.remove(index);
                        }
                        VecDiff::Move {
                            old_index,
                            new_index,
                        } => {
                            if old_index != new_index {
                                fill_moves(&this.inner, old_index, new_index, &mut pending);
                                let value = this.inner.remove(old_index);
                                this.inner.insert(new_index, value);
                            }
                        }
                        VecDiff::Push { value } => {
                            this.inner.push(FlattenState::new(value));
                        }
                        VecDiff::Pop {} => {
                            let len = this.inner.pop().unwrap().len;
                            (0..len).for_each(|_| pending.push(VecDiff::Pop {}));
                        }
                        VecDiff::Clear {} => {
                            this.inner.clear();
                            return Poll::Ready(Some(VecDiff::Clear {}));
                        }
                    }

                    continue;
                }
                Some(Poll::Ready(None)) => {
                    this.signal.set(None);
                    true
                }
                Some(Poll::Pending) => false,
                None => true,
            };
        };

        let mut inner_done = true;
        let mut prev_len = 0;
        for state in this.inner.iter_mut() {
            inner_done &= state.poll_pending(cx, prev_len, &mut pending);
            prev_len += state.len;
        }

        if let Some(first) = pending.first {
            *this.pending = pending.rest;
            Poll::Ready(Some(first))
        } else if inner_done && top_done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

struct PendingBuilder<A> {
    first: Option<A>,
    rest: VecDeque<A>,
}

impl<A> PendingBuilder<A> {
    fn new() -> Self {
        Self {
            first: None,
            rest: VecDeque::new(),
        }
    }

    fn push(&mut self, value: A) {
        if let None = self.first {
            self.first = Some(value);
        } else {
            self.rest.push_back(value);
        }
    }
}
