use std::collections::BTreeMap;

use kobold_bit_buf::BitReader;
use kobold_types::{PropertyFlags, TypeDef};
use smartstring::alias::String;

use super::{property, utils, Error, SerializerFlags, SerializerParts, TypeTag};
use crate::{value::Object, Value};

pub fn deserialize<T: TypeTag>(
    de: &mut SerializerParts,
    reader: &mut BitReader<'_>,
) -> Result<Value, Error> {
    de.with_recursion_limit(|de| {
        reader.realign_to_byte();

        let types = de.types.clone();
        let res = match T::identity(reader, &types) {
            // If a type definition exists, read the full object.
            Ok(Some(type_def)) => {
                let object_size = read_bit_size(de, reader)? as usize;
                deserialize_properties::<T>(de, object_size, type_def, reader)?
            }

            // If we encountered a null pointer, return an empty value.
            Ok(None) => Value::Empty,

            // If no type definition exists but we're allowed to skip it,
            // consume the bits the object is supposed to occupy.
            Err(_) if de.options.skip_unknown_types => {
                log::error!("Encountered unknown type; skipping it");

                // When skipping an object at any position, it means that
                // we either start with a new aligned object or reach EOF.
                // In either case, we have to consume whole bytes anyway.
                let object_size = read_bit_size(de, reader)? as usize;
                reader.read_bytes(utils::bits_to_bytes(object_size))?;

                Value::Empty
            }

            // If no type definition was found but we're also not allowed
            // to skip the object, return an error.
            Err(e) => return Err(e),
        };

        Ok(res)
    })
}

fn deserialize_properties<T: TypeTag>(
    de: &mut SerializerParts,
    object_size: usize,
    type_def: &TypeDef,
    reader: &mut BitReader<'_>,
) -> Result<Value, Error> {
    let mut inner = BTreeMap::new();

    if de.options.shallow {
        deserialize_properties_shallow::<T>(&mut inner, de, type_def, reader)?;
    } else {
        deserialize_properties_deep::<T>(&mut inner, de, object_size, type_def, reader)?;
    }

    Ok(Value::Object(Object { inner }))
}

#[inline]
fn deserialize_properties_shallow<T: TypeTag>(
    obj: &mut BTreeMap<String, Value>,
    de: &mut SerializerParts,
    type_def: &TypeDef,
    reader: &mut BitReader<'_>,
) -> Result<(), Error> {
    // In shallow mode, we walk masked properties in order.
    let mask = de.options.property_mask;
    for property in type_def
        .properties
        .iter()
        .filter(|p| p.flags.contains(mask) && !p.flags.contains(PropertyFlags::DEPRECATED))
    {
        if property.flags.contains(PropertyFlags::DELTA_ENCODE)
            && !utils::read_bool(reader)?
            && de
                .options
                .flags
                .contains(SerializerFlags::FORBID_DELTA_ENCODE)
        {
            return Err(Error::MissingDelta);
        }

        let value = property::deserialize::<T>(de, property, reader)?;

        obj.insert(property.name.clone(), value);
    }

    Ok(())
}

#[inline]
fn deserialize_properties_deep<T: TypeTag>(
    obj: &mut BTreeMap<String, Value>,
    de: &mut SerializerParts,
    mut object_size: usize,
    type_def: &TypeDef,
    reader: &mut BitReader<'_>,
) -> Result<(), Error> {
    // In deep mode, the properties name themselves.
    while object_size > 0 {
        // Back up the current buffer length and read the property size.
        // This will also count padding bits to byte boundaries.
        let previous_buf_len = reader.remaining_bits();
        let property_size = utils::read_bits(reader, u32::BITS)? as usize;

        // Read the property's hash and find the object in type defs.
        let property_hash = utils::read_bits(reader, u32::BITS)? as u32;
        let property = type_def
            .properties
            .iter()
            .find(|p| p.hash == property_hash)
            .ok_or_else(|| Error::UnknownProperty(property_hash))?;

        // Deserialize the property's value.
        let value = property::deserialize::<T>(de, property, reader)?;

        // Validate the size expectations.
        let actual_size = previous_buf_len - reader.remaining_bits();
        if property_size != actual_size {
            return Err(Error::PropertySizeMismatch {
                expected: property_size,
                actual: actual_size,
            });
        }

        // Prepare for the next round of deserialization.
        object_size = object_size
            .checked_sub(property_size)
            .ok_or_else(|| Error::ObjectSizeMismatch)?;
        reader.realign_to_byte();

        // Lastly, insert the property into the object.
        obj.insert(property.name.clone(), value);
    }

    Ok(())
}

#[inline]
pub(crate) fn read_bit_size(
    de: &SerializerParts,
    reader: &mut BitReader<'_>,
) -> Result<u32, Error> {
    (!de.options.shallow)
        .then(|| Ok(utils::read_bits(reader, u32::BITS)? as u32 - u32::BITS))
        .unwrap_or(Ok(0))
}
