#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class EXAMPLEPLUGIN_API FExamplePluginModule : public IModuleInterface
{
public:

    /** IModuleInterface implementation */
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;
    
    /** Get the singleton instance of this module */
    static FExamplePluginModule& Get()
    {
        return FModuleManager::LoadModuleChecked<FExamplePluginModule>("ExamplePlugin");
    }
    
    /** Check if this module is loaded and ready to use */
    static bool IsAvailable()
    {
        return FModuleManager::Get().IsModuleLoaded("ExamplePlugin");
    }
};

/**
 * Utility class providing helper functions for other plugins
 */
class EXAMPLEPLUGIN_API FExamplePluginUtils
{
public:
    /** Generate a random color */
    static FLinearColor GetRandomColor();
    
    /** Format a number with commas for display */
    static FString FormatNumberWithCommas(int32 Number);
    
    /** Get a greeting message with the current time */
    static FString GetGreetingMessage(const FString& Name = TEXT("World"));
    
    /** Calculate distance between two vectors */
    static float CalculateDistance(const FVector& A, const FVector& B);
};