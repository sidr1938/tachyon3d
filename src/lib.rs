use std::any::{Any, TypeId};
// Try to maintain > 2 dependencies
use fxhash::FxHashMap;

// RENAME TO TACHYON3D-T3D API
pub type System = Box<dyn FnMut() + 'static>;
pub trait Runtime {
    fn run_plugin(self, app: AppT3D);
}

pub trait Scheduler {
    fn systems(&mut self) -> &mut Vec<System>;
    fn add_system(&mut self, system: System);
}

pub trait Plugin where Self: 'static {
    fn build(self, app: &mut AppT3D) where Self: Sized {
        app.resources.insert(self);
    }
}

pub struct SchedulerHandler {
    pub cached_scheduler: Option<SchedulerInUse>,
    pub internal: FxHashMap<TypeId, Box<dyn Scheduler>>,
}

pub struct SchedulerInUse {
    label: TypeId,
    pub scheduler: Box<dyn Scheduler>,
}

impl SchedulerHandler {
    pub fn get_direct(&self, label: TypeId) -> Option<&Box<dyn Scheduler>> {
        if let Some(scheduler) = &self.cached_scheduler {
            if scheduler.label == label {
                return Some(&scheduler.scheduler);
            }
        }
        self.internal.get(&label)
    }
    pub fn get_mut_direct(&mut self, label: TypeId) -> Option<&mut Box<dyn Scheduler>> {
        if let Some(scheduler) = &mut self.cached_scheduler {
            if scheduler.label == label {
                return Some(&mut scheduler.scheduler);
            }
        }
        self.internal.get_mut(&label)
    }
    pub fn get<T: 'static>(&self) -> Option<&Box<dyn Scheduler>> {
        self.get_direct(TypeId::of::<T>())
    }
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut Box<dyn Scheduler>> {
        self.get_mut_direct(TypeId::of::<T>())
    }
}
pub struct AppT3D {
    pub schedulers: SchedulerHandler,
    pub resources: ResourceHandler
}

impl AppT3D {
    pub fn new() -> Self {
        Self {
            schedulers: SchedulerHandler {
                cached_scheduler: None,
                internal: FxHashMap::default(),
            },
            resources: ResourceHandler {
                cache: None,
                internal: FxHashMap::default()
            },
        }
    }

    // Could make a trait for this
    pub fn cache_scheduler<T: 'static>(&mut self) -> &mut Self {
        if let Some(old_scheduler) = self.schedulers.cached_scheduler.take() {
            self.schedulers.internal.insert(old_scheduler.label, old_scheduler.scheduler);
        }
        self.schedulers.cached_scheduler = Some(SchedulerInUse {
            label: TypeId::of::<T>(),
            scheduler: self.schedulers.internal.remove(&TypeId::of::<T>()).expect("No schedular exists")
        });
        self
    }
    pub fn cache_resource<T: 'static>(&mut self) -> &mut Self {
        if let Some(old_resource) = self.resources.cache.take() {
            self.resources.internal.insert(old_resource.label, old_resource.resource);
        }
        self.resources.cache = Some(ResourceInUse {
            label: TypeId::of::<T>(),
            resource: self.resources.internal.remove(&TypeId::of::<T>()).expect("No resource exists"),
        });
        self
    }


    pub fn add_systems<T: 'static, F: FnMut() + 'static>(&mut self, label: T, systems: Vec<F>) -> &mut Self {
        // Cached
        if let Some(active_scheduler) = &mut self.schedulers.cached_scheduler {
            if active_scheduler.label == label.type_id() {
                for system in systems {
                    self.schedulers.cached_scheduler.as_mut().unwrap().scheduler.add_system(Box::new(system));
                }
                return self;
            }
        }
        let scheduler = self.schedulers.internal.get_mut(&label.type_id()).unwrap();
        for system in systems {
            scheduler.add_system(Box::new(system));
        }
        self
    }
    pub fn add_scheduler<T: 'static>(&mut self, scheduler: Box<dyn Scheduler>) -> &mut Self {
        self.schedulers.internal.insert(TypeId::of::<T>(), scheduler);
        self
    }
    // Build on top of code
    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn run<R: Runtime + 'static>(mut self){
        self.resources.remove::<R>().unwrap().run_plugin(self);
    }
}

// Resource Handler Stuff
pub struct ResourceInUse {
    label: TypeId,
    pub resource: Box<dyn Any>,
}
pub struct ResourceHandler {
    pub cache: Option<ResourceInUse>,
    pub internal: FxHashMap<TypeId, Box<dyn Any>>,
}
impl ResourceHandler {
    pub fn new() -> Self {
        Self {
            cache: None,
            internal: Default::default(),
        }
    }
    pub fn insert<T: Any>(&mut self, resource: T) {
        self.internal.insert(resource.type_id(), Box::new(resource));
    }
    pub fn get<T: 'static>(&self) -> Option<&T> {
        if let Some(cached_resource) = &self.cache {
            if cached_resource.label == TypeId::of::<T>() {
                return cached_resource.resource.downcast_ref();
            }
        }
        self.internal.get(&TypeId::of::<T>()).and_then(|r| r.downcast_ref())
    }
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        if let Some(cached_resource) = &mut self.cache {
            if cached_resource.label == TypeId::of::<T>() {
                return cached_resource.resource.downcast_mut();
            }
        }
        self.internal.get_mut(&TypeId::of::<T>())
            .and_then(|r| r.downcast_mut())
    }
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        if let Some(cached_resource) = self.cache.take() {
            if cached_resource.label == TypeId::of::<T>() {
                return cached_resource.resource.downcast().ok().map(|r| *r);
            }
        }
        self.internal.remove(&TypeId::of::<T>()).and_then(|f| f.downcast::<T>()
            .ok().map(|r| *r))
    }
}



