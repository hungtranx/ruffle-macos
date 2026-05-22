use crate::avm2::Error;
use crate::avm2::activation::Activation;
use crate::avm2::parameters::ParametersExt;
use crate::avm2::value::Value;

pub use crate::avm2::object::dictionary_allocator;

/// Implements `Dictionary`'s instance initializer.
pub fn dictionary_initializer<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Value<'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let weak_keys = args.get_bool(0);

    if let Some(dictionary) = this
        .as_object()
        .and_then(|object| object.as_dictionary_object())
    {
        dictionary.set_weak_keys(weak_keys, activation.gc());
    }

    Ok(Value::Undefined)
}
