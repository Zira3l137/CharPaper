use gltf::json;
use gltf::json::accessor::ComponentType;
use gltf::json::accessor::GenericComponentType;
use gltf::json::accessor::Type;
use gltf::json::validation::Checked::Valid;
use gltf::json::validation::USize64;

/// A glTF document and the binary chunk its accessors point into, written as one .glb.
pub(crate) struct Glb {
    pub root: json::Root,
    /// Top-level JSON written as it is, over what `root` has under the same key.
    pub raw: serde_json::Map<String, serde_json::Value>,
    bin: Vec<u8>,
}

impl Glb {
    pub fn new() -> Self {
        let mut root = json::Root::default();
        root.asset.generator = Some(format!("CharPaper {}", env!("CARGO_PKG_VERSION")));
        Self { root, raw: serde_json::Map::new(), bin: Vec::new() }
    }

    /// Floats as an accessor of `kind`; `bounds` gives it its min and max, which glTF asks of
    /// positions and animation times.
    pub fn floats(
        &mut self,
        values: &[f32],
        kind: Type,
        bounds: bool,
    ) -> json::Index<json::Accessor> {
        let width = kind.multiplicity();
        let count = values.len() / width;
        let (min, max) = match bounds {
            false => (None, None),
            true => {
                let mut min = vec![f32::INFINITY; width];
                let mut max = vec![f32::NEG_INFINITY; width];
                for chunk in values.chunks_exact(width) {
                    for (i, &v) in chunk.iter().enumerate() {
                        min[i] = min[i].min(v);
                        max[i] = max[i].max(v);
                    }
                }
                (Some(serde_json::json!(min)), Some(serde_json::json!(max)))
            }
        };
        let view = self.view(values.iter().flat_map(|v| v.to_le_bytes()).collect());
        self.root.push(json::Accessor {
            buffer_view: Some(view),
            byte_offset: None,
            count: USize64::from(count),
            component_type: Valid(GenericComponentType(ComponentType::F32)),
            extensions: None,
            extras: Default::default(),
            type_: Valid(kind),
            min,
            max,
            name: None,
            normalized: false,
            sparse: None,
        })
    }

    /// Integers as an accessor of `kind`, in the smallest of `component` glTF allows.
    pub fn integers(
        &mut self,
        values: &[u32],
        kind: Type,
        component: ComponentType,
    ) -> json::Index<json::Accessor> {
        let bytes: Vec<u8> = match component {
            ComponentType::U8 => values.iter().map(|&v| v as u8).collect(),
            ComponentType::U16 => values.iter().flat_map(|&v| (v as u16).to_le_bytes()).collect(),
            _ => values.iter().flat_map(|v| v.to_le_bytes()).collect(),
        };
        let view = self.view(bytes);
        self.root.push(json::Accessor {
            buffer_view: Some(view),
            byte_offset: None,
            count: USize64::from(values.len() / kind.multiplicity()),
            component_type: Valid(GenericComponentType(component)),
            extensions: None,
            extras: Default::default(),
            type_: Valid(kind),
            min: None,
            max: None,
            name: None,
            normalized: false,
            sparse: None,
        })
    }

    pub fn view(&mut self, bytes: Vec<u8>) -> json::Index<json::buffer::View> {
        let offset = self.bin.len();
        self.bin.extend(bytes);
        let length = self.bin.len() - offset;
        // glTF asks an accessor's data to start on a multiple of its component's size, 4 at most.
        self.bin.resize(self.bin.len().next_multiple_of(4), 0);
        self.root.push(json::buffer::View {
            buffer: json::Index::new(0),
            byte_length: USize64::from(length),
            byte_offset: Some(USize64::from(offset)),
            byte_stride: None,
            name: None,
            target: None,
            extensions: None,
            extras: Default::default(),
        })
    }

    pub fn into_vec(mut self) -> Vec<u8> {
        if !self.bin.is_empty() {
            self.root.push(json::Buffer {
                byte_length: USize64::from(self.bin.len()),
                name: None,
                uri: None,
                extensions: None,
                extras: Default::default(),
            });
        }
        let mut document =
            serde_json::to_value(&self.root).expect("a glTF document always serializes");
        if let Some(object) = document.as_object_mut() {
            object.extend(std::mem::take(&mut self.raw));
        }
        let json = serde_json::to_vec(&document).expect("a glTF document always serializes");
        let glb = gltf::binary::Glb {
            header: gltf::binary::Header { magic: *b"glTF", version: 2, length: 0 },
            json: json.into(),
            bin: (!self.bin.is_empty()).then(|| self.bin.into()),
        };
        glb.to_vec().expect("an in-memory .glb always writes")
    }
}
