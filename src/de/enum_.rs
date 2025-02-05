use serde::de;

use crate::de::{Deserializer, Error, Result};

pub(crate) struct UnitVariantAccess<'a, 'b, 's> {
    de: &'a mut Deserializer<'b, 's>,
}

impl<'a, 'b, 's> UnitVariantAccess<'a, 'b, 's> {
    pub(crate) fn new(de: &'a mut Deserializer<'b, 's>) -> Self {
        UnitVariantAccess { de }
    }
}

impl<'de> de::EnumAccess<'de> for UnitVariantAccess<'_, 'de, '_> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, 'a, 's> de::VariantAccess<'de> for UnitVariantAccess<'a, 'de, 's> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(Error::InvalidType)
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::InvalidType)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::InvalidType)
    }
}

pub(crate) struct VariantAccess<'a, 'b, 's> {
    de: &'a mut Deserializer<'b, 's>,
}

impl<'a, 'b, 's> VariantAccess<'a, 'b, 's> {
    pub(crate) fn new(de: &'a mut Deserializer<'b, 's>) -> Self {
        VariantAccess { de }
    }
}

impl<'de> de::EnumAccess<'de> for VariantAccess<'_, 'de, '_> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self.de)?;
        self.de.parse_object_colon()?;
        Ok((variant, self))
    }
}

impl<'de, 'a, 's> de::VariantAccess<'de> for VariantAccess<'a, 'de, 's> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        de::Deserialize::deserialize(self.de)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}
