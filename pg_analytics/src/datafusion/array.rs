use deltalake::arrow::{
    array::{
        Array, BooleanArray, BooleanBuilder, Decimal128Array, Float32Array, Float64Array,
        GenericByteBuilder, Int16Array, Int32Array, Int64Array, ListArray, ListBuilder,
        PrimitiveBuilder, StringArray, UInt32Array,
    },
    datatypes::{
        ArrowPrimitiveType, ByteArrayType, Decimal128Type, Float32Type, Float64Type,
        GenericStringType, Int16Type, Int32Type, Int64Type, UInt32Type,
    },
};

type Column<T> = Vec<Option<T>>;
type ColumnNested<T> = Vec<Option<Column<T>>>;

pub trait IntoArray<T, A>
where
    A: Array + FromIterator<Option<T>>,
    Self: IntoIterator<Item = Option<T>> + Sized,
{
    fn into_array(self) -> A {
        A::from_iter(self)
    }
}

pub trait IntoGenericBytesListArray<T, B>
where
    B: ByteArrayType,
    T: AsRef<B::Native>,
    Self: IntoIterator<Item = Option<Vec<Option<T>>>> + Sized,
{
    fn into_array(self) -> ListArray {
        let mut builder = ListBuilder::new(GenericByteBuilder::<B>::new());
        for opt_vec in self {
            if let Some(vec) = opt_vec {
                for opt_t in vec {
                    builder.values().append_option(opt_t);
                }
                builder.append(true);
            } else {
                builder.append(false);
            }
        }
        builder.finish()
    }
}

pub trait IntoBooleanListArray
where
    Self: IntoIterator<Item = Option<Vec<Option<bool>>>> + Sized,
{
    fn into_array(self) -> ListArray {
        let mut builder = ListBuilder::new(BooleanBuilder::new());
        for opt_vec in self {
            if let Some(vec) = opt_vec {
                for opt_t in vec {
                    builder.values().append_option(opt_t);
                }
                builder.append(true);
            } else {
                builder.append(false);
            }
        }
        builder.finish()
    }
}

pub trait IntoPrimitiveListArray<A>
where
    A: ArrowPrimitiveType,
    Self: IntoIterator<Item = Option<Vec<Option<A::Native>>>> + Sized,
{
    fn into_array(self) -> ListArray {
        let mut builder = ListBuilder::new(PrimitiveBuilder::<A>::new());
        for opt_vec in self {
            if let Some(vec) = opt_vec {
                for opt_t in vec {
                    builder.values().append_option(opt_t);
                }
                builder.append(true);
            } else {
                builder.append(false);
            }
        }
        builder.finish()
    }
}

impl IntoArray<bool, BooleanArray> for Column<bool> {}
impl IntoBooleanListArray for ColumnNested<bool> {}

impl IntoArray<String, StringArray> for Column<String> {}
impl IntoGenericBytesListArray<String, GenericStringType<i32>> for ColumnNested<String> {}

impl IntoArray<i16, Int16Array> for Column<i16> {}
impl IntoPrimitiveListArray<Int16Type> for ColumnNested<i16> {}

impl IntoArray<i32, Int32Array> for Column<i32> {}
impl IntoPrimitiveListArray<Int32Type> for ColumnNested<i32> {}

impl IntoArray<i64, Int64Array> for Column<i64> {}
impl IntoPrimitiveListArray<Int64Type> for ColumnNested<i64> {}

impl IntoArray<u32, UInt32Array> for Column<u32> {}
impl IntoPrimitiveListArray<UInt32Type> for ColumnNested<u32> {}

impl IntoArray<f32, Float32Array> for Column<f32> {}
impl IntoPrimitiveListArray<Float32Type> for ColumnNested<f32> {}

impl IntoArray<f64, Float64Array> for Column<f64> {}
impl IntoPrimitiveListArray<Float64Type> for ColumnNested<f64> {}

impl IntoArray<i128, Decimal128Array> for Column<i128> {}
impl IntoPrimitiveListArray<Decimal128Type> for ColumnNested<i128> {}
