//! Function object impl

use crate::avm2::activation::Activation;
use crate::avm2::function::Executable;
use crate::avm2::method::Method;
use crate::avm2::names::{Namespace, QName};
use crate::avm2::object::script_object::{ScriptObject, ScriptObjectData};
use crate::avm2::object::{ClassObject, Object, ObjectPtr, TObject};
use crate::avm2::scope::Scope;
use crate::avm2::value::Value;
use crate::avm2::Error;
use gc_arena::{Collect, GcCell, MutationContext};
use std::cell::{Ref, RefMut};

/// An Object which can be called to execute its function code.
#[derive(Collect, Debug, Clone, Copy)]
#[collect(no_drop)]
pub struct FunctionObject<'gc>(GcCell<'gc, FunctionObjectData<'gc>>);

#[derive(Collect, Debug, Clone)]
#[collect(no_drop)]
pub struct FunctionObjectData<'gc> {
    /// Base script object
    base: ScriptObjectData<'gc>,

    /// Executable code
    exec: Option<Executable<'gc>>,
}

impl<'gc> FunctionObject<'gc> {
    /// Construct a function from an ABC method and the current closure scope.
    ///
    /// This associated constructor will also create and initialize an empty
    /// `Object` prototype for the function.
    pub fn from_function(
        activation: &mut Activation<'_, 'gc, '_>,
        method: Method<'gc>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
    ) -> Result<Object<'gc>, Error> {
        let mut this = Self::from_method(activation, method, scope, None);
        let es3_proto = ScriptObject::object(
            activation.context.gc_context,
            activation.avm2().prototypes().object,
        );

        this.install_slot(
            activation.context.gc_context,
            QName::new(Namespace::public(), "prototype"),
            0,
            es3_proto.into(),
            false,
        );

        Ok(this)
    }

    /// Construct a method from an ABC method and the current closure scope.
    ///
    /// The given `reciever`, if supplied, will override any user-specified
    /// `this` parameter.
    pub fn from_method(
        activation: &mut Activation<'_, 'gc, '_>,
        method: Method<'gc>,
        scope: Option<GcCell<'gc, Scope<'gc>>>,
        receiver: Option<Object<'gc>>,
    ) -> Object<'gc> {
        let fn_proto = activation.avm2().prototypes().function;
        let exec = Some(Executable::from_method(
            method,
            scope,
            receiver,
            activation.context.gc_context,
        ));

        FunctionObject(GcCell::allocate(
            activation.context.gc_context,
            FunctionObjectData {
                base: ScriptObjectData::base_new(Some(fn_proto), None),
                exec,
            },
        ))
        .into()
    }
}

impl<'gc> TObject<'gc> for FunctionObject<'gc> {
    fn base(&self) -> Ref<ScriptObjectData<'gc>> {
        Ref::map(self.0.read(), |read| &read.base)
    }

    fn base_mut(&self, mc: MutationContext<'gc, '_>) -> RefMut<ScriptObjectData<'gc>> {
        RefMut::map(self.0.write(mc), |write| &mut write.base)
    }

    fn as_ptr(&self) -> *const ObjectPtr {
        self.0.as_ptr() as *const ObjectPtr
    }

    fn to_string(&self, _mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        Ok("function Function() {}".into())
    }

    fn to_locale_string(&self, mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        self.to_string(mc)
    }

    fn value_of(&self, _mc: MutationContext<'gc, '_>) -> Result<Value<'gc>, Error> {
        Ok(Value::Object(Object::from(*self)))
    }

    fn as_executable(&self) -> Option<Executable<'gc>> {
        self.0.read().exec.clone()
    }

    fn call(
        self,
        receiver: Option<Object<'gc>>,
        arguments: &[Value<'gc>],
        activation: &mut Activation<'_, 'gc, '_>,
        subclass_object: Option<ClassObject<'gc>>,
    ) -> Result<Value<'gc>, Error> {
        if let Some(exec) = &self.0.read().exec {
            exec.exec(
                receiver,
                arguments,
                activation,
                subclass_object,
                self.into(),
            )
        } else {
            Err("Not a callable function!".into())
        }
    }

    fn construct(
        self,
        activation: &mut Activation<'_, 'gc, '_>,
        arguments: &[Value<'gc>],
    ) -> Result<Object<'gc>, Error> {
        let class: Object<'gc> = self.into();
        let prototype = self
            .get_property(
                class,
                &QName::new(Namespace::public(), "prototype").into(),
                activation,
            )?
            .coerce_to_object(activation)?;

        let instance = prototype.derive(activation)?;

        self.call(Some(instance), arguments, activation, None)?;

        Ok(instance)
    }

    fn derive(&self, activation: &mut Activation<'_, 'gc, '_>) -> Result<Object<'gc>, Error> {
        let this: Object<'gc> = Object::FunctionObject(*self);
        let base = ScriptObjectData::base_new(Some(this), None);

        Ok(FunctionObject(GcCell::allocate(
            activation.context.gc_context,
            FunctionObjectData { base, exec: None },
        ))
        .into())
    }
}
