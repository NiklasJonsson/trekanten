use std::path::Path;

use crate::pipeline::GraphicsPipeline;
use crate::vertex::VertexDescription;

pub struct Materials {

}

pub struct MaterialHandle {

}

pub struct Material {
    pipeline: GraphicsPipeline,
}

impl Material {
    pub fn pipeline(&self) -> &GraphicsPipeline {
        &self.pipeline
    }
}

pub struct MaterialDescriptor {
}

impl MaterialDescriptor {
    pub fn builder() -> MaterialDescriptorBuilder {
        unimplemented!()
    }
}

pub struct MaterialDescriptorBuilder {

}

impl MaterialDescriptorBuilder {
    pub fn vertex_shader<P: AsRef<Path>>(mut self, path: P) -> Self {
        unimplemented!()
    }

    pub fn fragment_shader<P: AsRef<Path>>(mut self, path: P) -> Self {
        unimplemented!()
    }

     pub fn vertex_type<V>(mut self) -> Self
        where
            V: VertexDescription,
    {
        unimplemented!()
    }

     pub fn build(self) -> MaterialDescriptor {
         unimplemented!()
     }
}



