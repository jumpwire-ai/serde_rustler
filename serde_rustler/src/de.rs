use crate::{atoms, error::Error, util};
use rustler::{
    types::{ListIterator, MapIterator},
    Term, TermType,
};
use serde::{
    de::{
        self, Deserialize, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
        Visitor,
    },
    forward_to_deserialize_any,
};
use std::iter;

/**
 * Converts a native Elixir term to a native Rust type.
 */
pub fn from_term<'de, 'a: 'de, T>(term: Term<'a>) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    let de = Deserializer::from(term);
    T::deserialize(de)
}

pub struct Deserializer<'a> {
    term: Term<'a>,
}

impl<'a> From<Term<'a>> for Deserializer<'a> {
    fn from(term: Term<'a>) -> Deserializer<'a> {
        Deserializer { term }
    }
}

macro_rules! try_parse_number {
    ($term:expr, $type:ty, $visitor:expr, $visit_fn:ident) => {
        if let Ok(num) = util::parse_number(&$term) as Result<$type, Error> {
            return $visitor.$visit_fn(num);
        }
    };
}

impl<'de, 'a: 'de> de::Deserializer<'de> for Deserializer<'a> {
    type Error = Error;

    // TODO
    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.term.get_type() {
            TermType::Atom => {
                if util::is_nil(&self.term) {
                    // unit (nil)
                    self.deserialize_unit(visitor)
                } else if let Ok(b) = util::parse_bool(&self.term) {
                    // bool (t, f)
                    visitor.visit_bool(b)
                } else {
                    // unit variant (atom)
                    let enum_type = EnumDeserializerType::Any;
                    let de = EnumDeserializer::new(enum_type, self.term, &[], None)?;
                    visitor.visit_enum(de)
                }
            }
            // i8, i16, i32, i64, u8, u16, u32, u64, f32, f64 (i128, u128)
            TermType::Number => {
                try_parse_number!(self.term, f64, visitor, visit_f64);
                try_parse_number!(self.term, i64, visitor, visit_i64);
                try_parse_number!(self.term, u64, visitor, visit_u64);

                Err(Error::ExpectedNumber)
            }
            // char
            // string
            // byte array
            TermType::Binary => self.deserialize_str(visitor),
            // seq
            TermType::EmptyList | TermType::List => self.deserialize_seq(visitor),
            // map
            // struct
            // struct variant
            TermType::Map => {
                let iter = MapIterator::new(self.term).ok_or(Error::ExpectedMap)?;
                let de = match util::validate_struct(&self.term, None) {
                    Err(_) => MapDeserializer::new(iter, None),
                    Ok(struct_name_term) => MapDeserializer::new(iter, Some(struct_name_term)),
                };

                visitor.visit_map(de)
            }
            // newtype struct
            // newtype variant (atom, len 2)
            // tuple struct (atom, len 3+)
            // tuple variant (atom, len 3+)
            // => if nothing else, tuple (any len)
            TermType::Tuple => self.deserialize_seq(visitor),
            _ => Err(Error::TypeHintsRequired),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if util::is_nil(&self.term) {
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNil)
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(util::parse_bool(&self.term)?)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if util::is_nil(&self.term) {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(util::parse_number(&self.term)?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(util::parse_number(&self.term)?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(util::parse_number(&self.term)?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(util::parse_number(&self.term)?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(util::parse_number(&self.term)?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(util::parse_number(&self.term)?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(util::parse_number(&self.term)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(util::parse_number(&self.term)?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(util::parse_number(&self.term)?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(util::parse_number(&self.term)?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        util::validate_binary(&self.term)?;
        util::term_to_str(&self.term)
            .or(Err(Error::ExpectedChar))
            .and_then(|string| {
                if string.len() == 1 {
                    visitor.visit_char(string.chars().next().unwrap())
                } else {
                    Err(Error::ExpectedChar)
                }
            })
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(util::parse_str(self.term)?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(util::parse_binary(self.term)?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(util::parse_binary(self.term)?)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tuple = util::validate_tuple(self.term, Some(2))?;
        let name_term =
            atoms::str_to_term(&self.term.get_env(), name).or(Err(Error::ExpectedStructName))?;

        if tuple[0].ne(&name_term) {
            return Err(Error::InvalidStructName);
        }

        visitor.visit_newtype_struct(Deserializer::from(tuple[1]))
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if !self.term.is_list() {
            return Err(Error::ExpectedList);
        }

        let iter: ListIterator = self.term.decode().or(Err(Error::ExpectedList))?;
        visitor.visit_seq(SequenceDeserializer::new(iter))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tuple = util::validate_tuple(self.term, Some(len))?;
        visitor.visit_seq(SequenceDeserializer::new(tuple.into_iter()))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut tuple = util::validate_tuple(self.term, Some(len + 1))?;
        let name_term =
            atoms::str_to_term(&self.term.get_env(), name).or(Err(Error::ExpectedStructName))?;

        if tuple[0].ne(&name_term) {
            return Err(Error::InvalidStructName);
        }

        let iter = tuple.split_off(1).into_iter();
        visitor.visit_seq(SequenceDeserializer::new(iter))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if !self.term.is_map() {
            return Err(Error::ExpectedMap);
        }

        let iter = MapIterator::new(self.term).ok_or(Error::ExpectedMap)?;
        visitor.visit_map(MapDeserializer::new(iter, None))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let struct_name_term = util::validate_struct(&self.term, Some(name))?;
        let iter = MapIterator::new(self.term).ok_or(Error::ExpectedStruct)?;
        visitor.visit_map(MapDeserializer::new(iter, Some(struct_name_term)))
    }

    // could be...?
    //  - unit variant      [enum var]              (atom)
    //  - newtype variant   [enum var + val]        (tuple(var, T))
    //  - tuple variant     [enum var + vals...]    (tuple(var, vals...))
    //  - struct variant    [enum var + struct]     (struct(var, vals...))
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use EnumDeserializerType as EnumType;

        let variant: Option<(EnumType, Term<'a>)> = match self.term.get_type() {
            // unit variant
            TermType::Atom => Some((EnumType::Unit, self.term)),
            TermType::Binary => Some((EnumType::Unit, self.term)),
            TermType::Number => Some((EnumType::Unit, self.term)),
            // newtype or tuple variant
            TermType::Tuple => {
                let tuple = util::validate_tuple(self.term, None)?;
                match tuple.len() {
                    0 | 1 => None,
                    2 => Some((EnumType::Newtype, tuple[0])),
                    _ => Some((EnumType::Tuple, tuple[0])),
                }
            }
            // struct variant
            TermType::Map => {
                let struct_name_term = util::validate_struct(&self.term, None)?;
                Some((EnumType::Struct, struct_name_term))
            }
            _ => None,
        };

        variant.ok_or(Error::ExpectedEnum).and_then(|variant| {
            let (vtype, term) = variant;
            let enum_de = EnumDeserializer::new(vtype, term, variants, Some(self.term))?;
            visitor.visit_enum(enum_de)
        })
    }

    // TODO: is this right?
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.term.get_type() {
            TermType::Atom => self.deserialize_str(visitor),
            TermType::Binary => self.deserialize_str(visitor),
            TermType::Number => self.deserialize_i64(visitor),
            _ => Err(Error::ExpectedAtom),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Just skip over this by calling visit_unit.
        visitor.visit_unit()
    }
}

struct SequenceDeserializer<'a, I>
where
    I: Iterator<Item = Term<'a>>,
{
    iter: iter::Fuse<I>,
}

impl<'a, I> SequenceDeserializer<'a, I>
where
    I: Iterator<Item = Term<'a>>,
{
    fn new(iter: I) -> Self {
        SequenceDeserializer { iter: iter.fuse() }
    }
}

impl<'de, 'a: 'de, I> SeqAccess<'de> for SequenceDeserializer<'a, I>
where
    I: Iterator<Item = Term<'a>>,
{
    type Error = Error;

    fn next_element_seed<V>(&mut self, seed: V) -> Result<Option<V::Value>, Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            None => Ok(None),
            Some(term) => seed.deserialize(Deserializer::from(term)).map(Some),
        }
    }
}

struct MapDeserializer<'a, I>
where
    I: Iterator,
{
    struct_name_term: Option<Term<'a>>,
    iter: iter::Fuse<I>,
    current_value: Option<Term<'a>>,
}

impl<'a, I> MapDeserializer<'a, I>
where
    I: Iterator,
{
    fn new(iter: I, struct_name_term: Option<Term<'a>>) -> Self {
        MapDeserializer {
            struct_name_term,
            iter: iter.fuse(),
            current_value: None,
        }
    }
}

impl<'de, 'a: 'de, I> MapAccess<'de> for MapDeserializer<'a, I>
where
    I: Iterator<Item = (Term<'a>, Term<'a>)>,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.current_value.is_some() {
            panic!("MapDeserializer.next_key_seed was called twice in a row")
        }

        self.iter
            .next()
            .and_then(|pair| match pair {
                (key, _) if atoms::__struct__().eq(&key) => self.iter.next(),
                pair => Some(pair),
            })
            .map_or(Ok(None), |pair| {
                let (key, value) = pair;
                self.current_value = Some(value);

                if let Some(_) = self.struct_name_term {
                    seed.deserialize(VariantNameDeserializer::from(key))
                        .map(Some)
                } else {
                    seed.deserialize(Deserializer::from(key)).map(Some)
                }
            })
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.current_value {
            None => Err(Error::ExpectedStructValue),
            Some(value) => {
                self.current_value = None;
                seed.deserialize(Deserializer::from(value))
            }
        }
    }
}

enum EnumDeserializerType {
    Any,
    Unit,
    Newtype,
    Tuple,
    Struct,
}

struct EnumDeserializer<'a> {
    variant_type: EnumDeserializerType,
    variant_term: Term<'a>,
    variant: String,
    term: Option<Term<'a>>,
}

impl<'a> EnumDeserializer<'a> {
    fn new(
        variant_type: EnumDeserializerType,
        variant_term: Term<'a>,
        variants: &'static [&'static str],
        term: Option<Term<'a>>,
    ) -> Result<Self, Error> {
        let var_de = VariantNameDeserializer::from(variant_term);
        let variant = String::deserialize(var_de).or(Err(Error::InvalidVariantName))?;

        match variant_type {
            EnumDeserializerType::Any => Ok(EnumDeserializer {
                variant_type,
                variant_term,
                variant,
                term,
            }),
            _ => {
                if variants.contains(&variant.as_str()) {
                    Ok(EnumDeserializer {
                        variant_type,
                        variant_term,
                        variant,
                        term,
                    })
                } else {
                    Err(Error::InvalidVariantName)
                }
            }
        }
    }
}

impl<'de, 'a: 'de> EnumAccess<'de> for EnumDeserializer<'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let var_de = VariantNameDeserializer::from(self.variant_term);
        let val = seed.deserialize(var_de)?;
        Ok((val, self))
    }
}

impl<'de, 'a: 'de> VariantAccess<'de> for EnumDeserializer<'a> {
    type Error = Error;

    // is an atom or string
    fn unit_variant(self) -> Result<(), Error> {
        match self.variant_type {
            EnumDeserializerType::Any | EnumDeserializerType::Unit => Ok(()),
            _ => Err(Error::ExpectedUnitVariant),
        }
    }

    // is a tagged (len 2) tuple (starting with atom or string)
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.variant_type {
            EnumDeserializerType::Any | EnumDeserializerType::Newtype => {
                if let Some(term) = self.term {
                    let tuple = util::validate_tuple(term, Some(2))?;
                    seed.deserialize(Deserializer::from(tuple[1]))
                } else {
                    Err(Error::ExpectedNewtypeVariant)
                }
            }
            _ => Err(Error::ExpectedNewtypeVariant),
        }
    }

    // is a tagged (len 3+) tuple (starting with atom or string)
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.variant_type {
            EnumDeserializerType::Any | EnumDeserializerType::Tuple => {
                if let Some(term) = self.term {
                    let mut tuple = util::validate_tuple(term, Some(len + 1))?;
                    let iter = tuple.split_off(1).into_iter();
                    visitor.visit_seq(SequenceDeserializer::new(iter))
                } else {
                    Err(Error::ExpectedTupleVariant)
                }
            }
            _ => Err(Error::ExpectedTupleVariant),
        }
    }

    // is a struct, with atom or string struct_name
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.variant_type {
            EnumDeserializerType::Struct => {
                if let Some(term) = self.term {
                    util::validate_struct(&term, Some(&self.variant))?;
                    let iter = MapIterator::new(term).ok_or(Error::ExpectedStruct)?;
                    visitor.visit_map(MapDeserializer::new(iter, Some(self.variant_term)))
                } else {
                    Err(Error::ExpectedStructVariant)
                }
            }
            _ => Err(Error::ExpectedStructVariant),
        }
    }
}

/// Deserializer for atoms and map keys.
struct VariantNameDeserializer<'a> {
    variant: Term<'a>,
}

impl<'a> From<Term<'a>> for VariantNameDeserializer<'a> {
    fn from(variant: Term<'a>) -> Self {
        VariantNameDeserializer { variant }
    }
}

impl<'de, 'a: 'de> de::Deserializer<'de> for VariantNameDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.variant.get_type() {
            TermType::Atom => {
                let string =
                    atoms::term_to_str(&self.variant).or(Err(Error::InvalidVariantName))?;
                visitor.visit_string(string)
            }
            TermType::Binary => visitor.visit_string(util::term_to_str(&self.variant)?),
            TermType::Number => visitor.visit_string(util::term_to_str(&self.variant)?),
            _ => Err(Error::ExpectedStringable),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
    }
}
