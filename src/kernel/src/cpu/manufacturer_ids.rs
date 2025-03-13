#[allow(non_camel_case_types)]

/*
All Manufacturer Ids from this page

https://en.wikipedia.org/wiki/CPUID

Currently not in use right now. Thought I may need it.

*/

// _ represent spaces for the enums

pub enum ManufacturerIds {
	AuthenticAMD, // AMD
	CentaurHauls, // IDT WinChip/Centaur (Including some VIA and Zhaoxin CPUs)
	CyrixInstead, // Cyrix/early STMicroelectronics and IBM
	GenuineIntel, // Intel
	GenuineIotel, // Intel (rare)
	TransmetaCPU, // Transmeta
	GenuineTMx86, // Transmeta
	Geode_By_NSC, // National Semiconductor
	NexGenDriven, // NexGen
	RiseRiseRise, // Rise
	SiS_SiS_SiS_, // SiS
	UMC_UMC_UMC_, // UMC
	Vortex86_SoC, // DM&P Vortex86
	__Shanghai__, // Zhaoxin
	HygonGenuine, // Hygon
	Genuine__RDC, // RDC Semiconductor Co. Ltd.
	E2K_MACHINE,  // MCST Elbrus
	VIA_VIA_VIA_, // VIA
	AMD_ISBETTER, // early engineering samples of AMD K5 processor

	// soft CPU Cores
	GenuineAO486, // ao486 CPU (old)
	MiSTer_AO486, // ao486 CPU (new)
	// v586 core (identical to Intel ID)

	// VM ID Cores
	MicrosoftXTA, // Microsoft x86-to-ARM
	// Apple Rosetta 2 (identical to Intel ID)
	VirtualApple, // newer versions of Apple Rosetta 2
	PowerVM_Lx86, // PowerVM Lx86 (x86 emulator for IBM POWER5/POWER6 processors)
	Neko_Project  /* Neko Project II (PC-98 emulator) (used when the CPU to emulate is set to
	               * "Neko Processor II") */
}
