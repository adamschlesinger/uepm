#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class DEPENDENCYPLUGIN_API FDependencyPluginModule : public IModuleInterface
{
public:

    /** IModuleInterface implementation */
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;
    
    /** Get the singleton instance of this module */
    static FDependencyPluginModule& Get()
    {
        return FModuleManager::LoadModuleChecked<FDependencyPluginModule>("DependencyPlugin");
    }
    
    /** Check if this module is loaded and ready to use */
    static bool IsAvailable()
    {
        return FModuleManager::Get().IsModuleLoaded("DependencyPlugin");
    }
};

/**
 * Enhanced functionality that builds on ExamplePlugin utilities
 */
class DEPENDENCYPLUGIN_API FDependencyPluginFeatures
{
public:
    /** Create a colorful status message using ExamplePlugin utilities */
    static FString CreateColorfulStatusMessage(const FString& Status, int32 Value);
    
    /** Generate a random spawn location within bounds */
    static FVector GetRandomSpawnLocation(const FVector& Center, float Radius);
    
    /** Create a formatted report using multiple ExamplePlugin functions */
    static FString GenerateSystemReport();
    
    /** Demonstrate plugin dependency by calling ExamplePlugin functions */
    static void DemonstratePluginIntegration();
};