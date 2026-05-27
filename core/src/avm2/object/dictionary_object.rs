//! Object representation for `flash.utils.Dictionary`

use crate::avm2::Error;
use crate::avm2::activation::Activation;
use crate::avm2::dynamic_map::DynamicKey;
use crate::avm2::object::script_object::ScriptObjectData;
use crate::avm2::object::{ClassObject, Object, TObject, WeakObject};
use crate::avm2::value::Value;
use crate::string::AvmString;
use core::fmt;
use gc_arena::barrier::unlock;
use gc_arena::collect::Trace;
use gc_arena::lock::{Lock, RefLock};
use gc_arena::{Collect, Gc, GcWeak, Mutation};
use ruffle_common::utils::HasPrefixField;
use std::cell::{Ref, RefMut};

/// A class instance allocator that allocates Dictionary objects.
pub fn dictionary_allocator<'gc>(
    class: ClassObject<'gc>,
    activation: &mut Activation<'_, 'gc>,
) -> Result<Object<'gc>, Error<'gc>> {
    let base = ScriptObjectData::new(class);

    Ok(DictionaryObject(Gc::new(
        activation.gc(),
        DictionaryObjectData {
            base,
            has_weak_keys: Lock::new(false),
            weak_entries: RefLock::new(WeakDictionaryMap::default()),
        },
    ))
    .into())
}

/// An object that allows associations between objects and values.
///
/// This is implemented by way of "object space", parallel to the property
/// space that ordinary properties live in. This space has no namespaces, and
/// keys are objects instead of strings.
#[derive(Clone, Collect, Copy)]
#[collect(no_drop)]
pub struct DictionaryObject<'gc>(pub Gc<'gc, DictionaryObjectData<'gc>>);

#[derive(Clone, Collect, Copy, Debug)]
#[collect(no_drop)]
pub struct DictionaryObjectWeak<'gc>(pub GcWeak<'gc, DictionaryObjectData<'gc>>);

impl fmt::Debug for DictionaryObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DictionaryObject")
            .field("ptr", &Gc::as_ptr(self.0))
            .finish()
    }
}

#[derive(Clone, HasPrefixField)]
#[repr(C, align(8))]
pub struct DictionaryObjectData<'gc> {
    /// Base script object
    base: ScriptObjectData<'gc>,

    has_weak_keys: Lock<bool>,

    weak_entries: RefLock<WeakDictionaryMap<'gc>>,
}

#[derive(Clone)]
struct WeakDictionaryEntry<'gc> {
    key: WeakObject<'gc>,
    value: Value<'gc>,
    enumerable: bool,
}

#[derive(Clone, Default)]
struct WeakDictionaryMap<'gc> {
    entries: Vec<Option<WeakDictionaryEntry<'gc>>>,
}

unsafe impl<'gc> Collect<'gc> for WeakDictionaryEntry<'gc> {
    fn trace<C: Trace<'gc>>(&self, cc: &mut C) {
        cc.trace(&self.key);
        cc.trace(&self.value);
    }
}

unsafe impl<'gc> Collect<'gc> for WeakDictionaryMap<'gc> {
    fn trace<C: Trace<'gc>>(&self, cc: &mut C) {
        for entry in self.entries.iter().flatten() {
            cc.trace(entry);
        }
    }
}

unsafe impl<'gc> Collect<'gc> for DictionaryObjectData<'gc> {
    fn trace<C: Trace<'gc>>(&self, cc: &mut C) {
        cc.trace(&self.base);
        cc.trace(&self.has_weak_keys);

        // Safe: tracing may remove stale weak entries, but it never adopts new
        // GC pointers without a write barrier.
        let mut weak_entries = unsafe { self.weak_entries.as_ref_cell() }.borrow_mut();
        weak_entries.trace_and_prune(cc);
    }
}

impl<'gc> WeakDictionaryMap<'gc> {
    fn trace_and_prune<C: Trace<'gc>>(&mut self, cc: &mut C) {
        self.entries.retain(|entry| {
            let Some(entry) = entry else {
                return false;
            };

            if entry.key.is_dropped() {
                return false;
            }

            cc.trace(&entry.key);
            cc.trace(&entry.value);
            true
        });
    }

    fn find_index(&self, key: Object<'gc>) -> Option<usize> {
        self.entries.iter().position(|entry| {
            entry.as_ref().is_some_and(|entry| {
                !entry.key.is_dropped() && std::ptr::eq(entry.key.as_ptr(), key.as_ptr())
            })
        })
    }

    fn get(&self, key: Object<'gc>) -> Option<Value<'gc>> {
        self.find_index(key)
            .and_then(|index| self.entries[index].as_ref())
            .map(|entry| entry.value)
    }

    fn insert(&mut self, key: Object<'gc>, value: Value<'gc>) {
        if let Some(index) = self.find_index(key) {
            if let Some(entry) = &mut self.entries[index] {
                entry.value = value;
            }
        } else if let Some(empty) = self.entries.iter_mut().find(|entry| entry.is_none()) {
            *empty = Some(WeakDictionaryEntry {
                key: key.downgrade(),
                value,
                enumerable: true,
            });
        } else {
            self.entries.push(Some(WeakDictionaryEntry {
                key: key.downgrade(),
                value,
                enumerable: true,
            }));
        }
    }

    fn remove(&mut self, key: Object<'gc>) {
        if let Some(index) = self.find_index(key) {
            self.entries[index] = None;
        }
    }

    fn contains_key(&self, key: Object<'gc>) -> bool {
        self.find_index(key).is_some()
    }

    fn next(&self, last_index: usize) -> Option<usize> {
        let mut public_index = 0;

        for entry in self.entries.iter().flatten() {
            if entry.enumerable && !entry.key.is_dropped() {
                public_index += 1;

                if public_index > last_index {
                    return Some(public_index);
                }
            }
        }

        None
    }

    fn entry_at(&self, index: usize) -> Option<&WeakDictionaryEntry<'gc>> {
        let mut public_index = 0;

        for entry in self.entries.iter().flatten() {
            if entry.enumerable && !entry.key.is_dropped() {
                public_index += 1;

                if public_index == index {
                    return Some(entry);
                }
            }
        }

        None
    }
}

impl<'gc> DictionaryObject<'gc> {
    pub fn set_weak_keys(self, weak_keys: bool, mc: &Mutation<'gc>) {
        unlock!(Gc::write(mc, self.0), DictionaryObjectData, has_weak_keys).set(weak_keys);
    }

    pub fn has_weak_keys(self) -> bool {
        self.0.has_weak_keys.get()
    }

    fn weak_entries(&self) -> Ref<'_, WeakDictionaryMap<'gc>> {
        self.0.weak_entries.borrow()
    }

    fn weak_entries_mut(&self, mc: &Mutation<'gc>) -> RefMut<'_, WeakDictionaryMap<'gc>> {
        unlock!(Gc::write(mc, self.0), DictionaryObjectData, weak_entries).borrow_mut()
    }

    /// Retrieve a value in the dictionary's object space.
    pub fn get_property_by_object(self, name: Object<'gc>) -> Value<'gc> {
        if self.has_weak_keys() {
            return self.weak_entries().get(name).unwrap_or(Value::Undefined);
        }

        self.base()
            .values()
            .get(&DynamicKey::Object(name))
            .map(|v| v.value)
            .unwrap_or(Value::Undefined)
    }

    /// Set a value in the dictionary's object space.
    pub fn set_property_by_object(self, name: Object<'gc>, value: Value<'gc>, mc: &Mutation<'gc>) {
        if self.has_weak_keys() {
            self.weak_entries_mut(mc).insert(name, value);
            return;
        }

        self.base()
            .values_mut(mc)
            .insert(DynamicKey::Object(name), value);
    }

    /// Delete a value from the dictionary's object space.
    pub fn delete_property_by_object(self, name: Object<'gc>, mc: &Mutation<'gc>) {
        if self.has_weak_keys() {
            self.weak_entries_mut(mc).remove(name);
            return;
        }

        self.base().values_mut(mc).remove(&DynamicKey::Object(name));
    }

    pub fn has_property_by_object(self, name: Object<'gc>) -> bool {
        if self.has_weak_keys() {
            return self.weak_entries().contains_key(name);
        }

        self.base().values().contains_key(&DynamicKey::Object(name))
    }

    fn base_enumerable_len(self) -> usize {
        self.base()
            .values()
            .iter()
            .filter(|(_, prop)| prop.enumerable)
            .count()
    }
}

impl<'gc> TObject<'gc> for DictionaryObject<'gc> {
    fn gc_base(&self) -> Gc<'gc, ScriptObjectData<'gc>> {
        HasPrefixField::as_prefix_gc(self.0)
    }

    // Calling `setPropertyIsEnumerable` on a `Dictionary` has no effect -
    // stringified properties are always enumerable.
    fn set_local_property_is_enumerable(
        &self,
        _mc: &Mutation<'gc>,
        _name: AvmString<'gc>,
        _is_enumerable: bool,
    ) {
    }

    fn get_enumerant_value(
        self,
        index: u32,
        _activation: &mut Activation<'_, 'gc>,
    ) -> Result<Value<'gc>, Error<'gc>> {
        if self.has_weak_keys() {
            let base_len = self.base_enumerable_len();
            if index as usize > base_len {
                return Ok(self
                    .weak_entries()
                    .entry_at(index as usize - base_len)
                    .map(|entry| entry.value)
                    .unwrap_or(Value::Undefined));
            }
        }

        Ok(*self
            .base()
            .values()
            .value_at(index as usize)
            .unwrap_or(&Value::Undefined))
    }

    fn get_next_enumerant(
        self,
        last_index: u32,
        _activation: &mut Activation<'_, 'gc>,
    ) -> Result<u32, Error<'gc>> {
        if !self.has_weak_keys() {
            return Ok(self.base().get_next_enumerant(last_index));
        }

        let next = self.base().get_next_enumerant(last_index);
        if next != 0 {
            return Ok(next);
        }

        let base_len = self.base_enumerable_len();
        let weak_last_index = (last_index as usize).saturating_sub(base_len);

        Ok(self
            .weak_entries()
            .next(weak_last_index)
            .map(|index| (base_len + index) as u32)
            .unwrap_or(0))
    }

    fn get_enumerant_name(
        self,
        index: u32,
        activation: &mut Activation<'_, 'gc>,
    ) -> Result<Value<'gc>, Error<'gc>> {
        if self.has_weak_keys() {
            let base_len = self.base_enumerable_len();
            if index as usize > base_len {
                return Ok(self
                    .weak_entries()
                    .entry_at(index as usize - base_len)
                    .and_then(|entry| entry.key.upgrade(activation.gc()))
                    .map(Value::Object)
                    .unwrap_or(Value::Null));
            }
        }

        Ok(self.base().get_enumerant_name(index).unwrap_or(Value::Null))
    }
}
