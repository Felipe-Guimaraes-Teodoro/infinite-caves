use chaos_vk::graphics::vk::{MemAllocators, Vk};
use std::sync::Arc;

use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo, QueueFlags};
use vulkano::image::ImageUsage;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use vulkano::VulkanLibrary;
use vulkano::instance::{Instance, InstanceCreateInfo};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub trait CustomNew {
    fn custom_new(el: &EventLoop<()>) -> Arc<Self>;
} 

impl CustomNew for Vk {
    fn custom_new(el: &EventLoop<()>) -> Arc<Self> {     
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        
        let required_extensions = Surface::required_extensions(el);
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        let window = Arc::new(WindowBuilder::new().build(&el).unwrap());
        let surface = Surface::from_window(instance.clone(), window.clone())
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        /* properly select a physical device */
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.contains(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|q| (p, q as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,

                _ => 4,
            })
            .expect("no device available");

    
        // for family in physical_device.queue_family_properties() {
        //     println!("Found a queue family with {:?} queue(s)", family.queue_count);
        // }
        
        let (device, mut queues) = Device::new(
                physical_device.clone(),
                DeviceCreateInfo {
                    queue_create_infos: vec![QueueCreateInfo {
                        queue_family_index,
                        ..Default::default()
                    }],
                    enabled_features: Features {
                        multi_draw_indirect: true,
                        ..Default::default()
                    },
                    enabled_extensions: device_extensions,
                    ..Default::default()
                },
            )
            .expect("failed to create device");

        let allocators = MemAllocators::new(device.clone());
    
        let queue = queues.next().unwrap();

        Arc::new(Self {
            queue,
            physical_device,
            device,
            queue_family_index,
            allocators: Arc::new(allocators),
            instance: instance,
            surface,
            window,
        })
    }
}