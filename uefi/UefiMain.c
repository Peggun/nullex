// uefi/uefi_boot.c  (EDK2-style loader that loads kernel to 1MiB)
#include <Uefi.h>
#include <Library/UefiLib.h>
#include <Library/MemoryAllocationLib.h>
#include <Library/UefiBootServicesTableLib.h>
#include <Protocol/SimpleFileSystem.h>
#include <Protocol/LoadedImage.h>
#include <Guid/FileInfo.h>
#include <Library/BaseMemoryLib.h>

typedef struct {
    VOID* memory_map;
    UINTN memory_map_size;
    UINTN descriptor_size;
    UINT32 descriptor_version;
    UINT64 physical_memory_offset; // key value we pass to kernel
    UINT64 kernel_entry_phys;      // physical entry point for kernel
} BOOT_INFO;

/* helper to get copy of memory map. Caller must free returned buffer if desired.
   Returns EFI_SUCCESS and writes map_key (the key to pass to ExitBootServices). */
EFI_STATUS get_and_copy_memory_map(
    VOID** out_map,
    UINTN* out_map_size,
    UINTN* out_descriptor_size,
    UINT32* out_descriptor_version,
    UINTN* out_map_key
) {
    EFI_STATUS Status;
    UINTN map_size = 0;
    UINTN map_key = 0;
    UINTN descriptor_size = 0;
    UINT32 descriptor_version = 0;
    EFI_MEMORY_DESCRIPTOR* map = NULL;

    /* Query size first (will return EFI_BUFFER_TOO_SMALL) */
    Status = gBS->GetMemoryMap(&map_size, map, &map_key, &descriptor_size, &descriptor_version);
    if (Status != EFI_BUFFER_TOO_SMALL) return Status;

    /* allocate slightly larger buffer to be safe */
    map_size += descriptor_size * 10;
    map = AllocatePool(map_size);
    if (!map) return EFI_OUT_OF_RESOURCES;

    Status = gBS->GetMemoryMap(&map_size, map, &map_key, &descriptor_size, &descriptor_version);
    if (EFI_ERROR(Status)) {
        FreePool(map);
        return Status;
    }

    *out_map = (VOID*)map;
    *out_map_size = map_size;
    *out_descriptor_size = descriptor_size;
    *out_descriptor_version = descriptor_version;
    *out_map_key = map_key;
    return EFI_SUCCESS;
}

/* Load the kernel binary file from the same device that loaded this UEFI app,
   allocate pages at 1MiB, copy the file there and return the physical entry. */
EFI_STATUS load_kernel_to_1m(EFI_HANDLE ImageHandle, UINT64* out_entry_phys, UINT64* out_file_size) {
    EFI_STATUS Status;
    EFI_LOADED_IMAGE_PROTOCOL *LoadedImage = NULL;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *SimpleFs = NULL;
    EFI_FILE_PROTOCOL *Root = NULL;
    EFI_FILE_PROTOCOL *KernelFile = NULL;
    EFI_FILE_INFO *FileInfo = NULL;
    UINTN FileInfoSize = 0;
    CHAR16 *KernelPath = L"\\BOOT\\KERNEL_X.BIN";
    EFI_PHYSICAL_ADDRESS LoadAddr = 0x00100000ULL; /* 1 MiB */
    UINTN Pages;
    UINTN ReadSize;

    /* Get LoadedImage protocol from our image handle */
    Status = gBS->HandleProtocol(ImageHandle, &gEfiLoadedImageProtocolGuid, (VOID**)&LoadedImage);
    if (EFI_ERROR(Status)) {
        Print(L"HandleProtocol(LoadedImage) failed: %r\n", Status);
        return Status;
    }

    /* Get SimpleFileSystem from the device that loaded this image */
    Status = gBS->HandleProtocol(LoadedImage->DeviceHandle, &gEfiSimpleFileSystemProtocolGuid, (VOID**)&SimpleFs);
    if (EFI_ERROR(Status)) {
        Print(L"HandleProtocol(SimpleFileSystem) failed: %r\n", Status);
        return Status;
    }

    Status = SimpleFs->OpenVolume(SimpleFs, &Root);
    if (EFI_ERROR(Status)) {
        Print(L"OpenVolume failed: %r\n", Status);
        return Status;
    }

    /* Open kernel file for read */
    Status = Root->Open(Root, &KernelFile, KernelPath, EFI_FILE_MODE_READ, 0);
    if (EFI_ERROR(Status)) {
        Print(L"Failed to open %s: %r\n", KernelPath, Status);
        return Status;
    }

    /* Query file info size first */
    FileInfoSize = 0;
    Status = KernelFile->GetInfo(KernelFile, &gEfiFileInfoGuid, &FileInfoSize, NULL);
    if (Status != EFI_BUFFER_TOO_SMALL) {
        /* If another error than buffer too small, try to continue - but usually we expect buffer too small */
        if (EFI_ERROR(Status)) {
            Print(L"KernelFile->GetInfo(alloc size) failed: %r\n", Status);
            KernelFile->Close(KernelFile);
            return Status;
        }
    }

    FileInfo = AllocatePool(FileInfoSize);
    if (!FileInfo) {
        KernelFile->Close(KernelFile);
        return EFI_OUT_OF_RESOURCES;
    }

    Status = KernelFile->GetInfo(KernelFile, &gEfiFileInfoGuid, &FileInfoSize, FileInfo);
    if (EFI_ERROR(Status)) {
        Print(L"KernelFile->GetInfo failed: %r\n", Status);
        FreePool(FileInfo);
        KernelFile->Close(KernelFile);
        return Status;
    }

    UINT64 kernel_size = FileInfo->FileSize;
    *out_file_size = kernel_size;

    /* Compute pages required and allocate at requested address */
    Pages = (UINTN)((kernel_size + 0xFFF) / 0x1000);
    if (Pages == 0) Pages = 1;

    /* Request pages at 1MiB */
    LoadAddr = 0x00100000ULL;
    Status = gBS->AllocatePages(AllocateAddress, EfiLoaderData, Pages, &LoadAddr);
    if (EFI_ERROR(Status)) {
        Print(L"AllocatePages at 0x00100000 failed: %r\n", Status);
        FreePool(FileInfo);
        KernelFile->Close(KernelFile);
        return Status;
    }

    /* Read file content into allocated area */
    ReadSize = (UINTN)kernel_size;
    Status = KernelFile->Read(KernelFile, &ReadSize, (VOID*)(UINTN)LoadAddr);
    if (EFI_ERROR(Status) || ReadSize != (UINTN)kernel_size) {
        Print(L"KernelFile->Read failed or short read: %r (read %u, expected %llu)\n", Status, ReadSize, kernel_size);
        FreePool(FileInfo);
        KernelFile->Close(KernelFile);
        return EFI_DEVICE_ERROR;
    }

    /* Try to parse ELF header for entry point (ELF64) */
    UINT8 *bytes = (UINT8*)(UINTN)LoadAddr;
    UINT64 entry = (UINT64)LoadAddr; /* default to load address */
    if (kernel_size >= 0x40 && bytes[0] == 0x7f && bytes[1] == 'E' && bytes[2] == 'L' && bytes[3] == 'F') {
        /* ELF64 e_entry is at offset 24 (8 bytes) in the ELF header for 64-bit */
        /* Ensure we are using little-endian (x86_64) */
        UINT64 raw_entry;
        /* Copy unaligned bytes safely */
        raw_entry = *(UINT64*)(bytes + 24);
        entry = raw_entry;
        Print(L"ELF detected. e_entry = 0x%lx\n", entry);
    } else {
        Print(L"Non-ELF kernel file, using load address 0x%lx as entry.\n", (UINT64)LoadAddr);
    }

    /* close and free file info */
    FreePool(FileInfo);
    KernelFile->Close(KernelFile);

    *out_entry_phys = entry;
    return EFI_SUCCESS;
}

EFI_STATUS EFIAPI UefiMain (IN EFI_HANDLE ImageHandle, IN EFI_SYSTEM_TABLE *SystemTable) {
    EFI_STATUS Status;
    UINT64 kernel_entry_phys = 0;
    UINT64 kernel_size = 0;

    /* Load kernel into 1MiB and compute physical entry */
    Status = load_kernel_to_1m(ImageHandle, &kernel_entry_phys, &kernel_size);
    if (EFI_ERROR(Status)) {
        Print(L"Failed to load kernel: %r\n", Status);
        return Status;
    }
    Print(L"Kernel loaded. size=%llu entry_phys=0x%lx\n", kernel_size, kernel_entry_phys);

    /* Now capture and copy the memory map (must be done just before ExitBootServices) */
    VOID* mem_map = NULL;
    UINTN mem_map_size = 0;
    UINTN desc_size = 0;
    UINT32 desc_version = 0;
    UINTN map_key = 0;

    Status = get_and_copy_memory_map(&mem_map, &mem_map_size, &desc_size, &desc_version, &map_key);
    if (EFI_ERROR(Status)) {
        Print(L"GetMemoryMap failed: %r\n", Status);
        return Status;
    }

    /* Prepare BootInfo in memory available to kernel after ExitBootServices */
    BOOT_INFO *bi = AllocatePool(sizeof(BOOT_INFO));
    if (!bi) {
        Print(L"AllocatePool for BOOT_INFO failed\n");
        return EFI_OUT_OF_RESOURCES;
    }
    bi->memory_map = mem_map;
    bi->memory_map_size = mem_map_size;
    bi->descriptor_size = desc_size;
    bi->descriptor_version = desc_version;

    /* Choose an offset. Option A: fixed constant (both kernel and loader agree) */
    const UINT64 PHYS_OFFSET = 0xFFFF800000000000ULL; /* example — must match kernel expectations */
    bi->physical_memory_offset = PHYS_OFFSET;
    bi->kernel_entry_phys = kernel_entry_phys;

    /* Exit boot services using the map_key returned earlier */
    Status = gBS->ExitBootServices(ImageHandle, map_key);
    if (EFI_ERROR(Status)) {
        Print(L"ExitBootServices failed: %r\n", Status);
        return Status;
    }

    /* Jump to kernel entry (physical address). Convert to function pointer and pass boot_info pointer.
       Kernel must accept a pointer to BOOT_INFO as its first argument. */
    typedef void (*kernel_entry_t)(BOOT_INFO*);
    kernel_entry_t kernel = (kernel_entry_t)(UINTN)bi->kernel_entry_phys;
    kernel(bi);

    /* We should never return */
    return EFI_SUCCESS;
}
