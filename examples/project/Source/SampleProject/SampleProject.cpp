// Copyright Epic Games, Inc. All Rights Reserved.

#include "SampleProject.h"
#include "Modules/ModuleManager.h"

// Include our NPM-distributed plugins
#include "ExamplePlugin.h"
#include "DependencyPlugin.h"

class FSampleProjectModule : public FDefaultGameModuleImpl
{
public:
    virtual void StartupModule() override
    {
        FDefaultGameModuleImpl::StartupModule();
        
        UE_LOG(LogTemp, Warning, TEXT("SampleProject: Game module starting up"));
        
        // Demonstrate using NPM-distributed plugins
        DemonstratePluginUsage();
    }
    
    virtual void ShutdownModule() override
    {
        UE_LOG(LogTemp, Warning, TEXT("SampleProject: Game module shutting down"));
        
        FDefaultGameModuleImpl::ShutdownModule();
    }

private:
    void DemonstratePluginUsage()
    {
        UE_LOG(LogTemp, Warning, TEXT("=== SampleProject Plugin Usage Demo ==="));
        
        // Check if our NPM plugins are available
        if (FExamplePluginModule::IsAvailable())
        {
            UE_LOG(LogTemp, Log, TEXT("✓ ExamplePlugin is available"));
            
            // Use ExamplePlugin directly
            FString Greeting = FExamplePluginUtils::GetGreetingMessage(TEXT("SampleProject"));
            UE_LOG(LogTemp, Log, TEXT("Greeting: %s"), *Greeting);
            
            // Generate some random colors
            for (int32 i = 0; i < 3; i++)
            {
                FLinearColor Color = FExamplePluginUtils::GetRandomColor();
                UE_LOG(LogTemp, Log, TEXT("Random Color %d: R=%.2f G=%.2f B=%.2f"), 
                       i + 1, Color.R, Color.G, Color.B);
            }
            
            // Format some numbers
            int32 Score = 1234567;
            FString FormattedScore = FExamplePluginUtils::FormatNumberWithCommas(Score);
            UE_LOG(LogTemp, Log, TEXT("Player Score: %s"), *FormattedScore);
        }
        else
        {
            UE_LOG(LogTemp, Error, TEXT("✗ ExamplePlugin is NOT available"));
        }
        
        if (FDependencyPluginModule::IsAvailable())
        {
            UE_LOG(LogTemp, Log, TEXT("✓ DependencyPlugin is available"));
            
            // Use DependencyPlugin features
            FString StatusMessage = FDependencyPluginFeatures::CreateColorfulStatusMessage(
                TEXT("GAME_READY"), 9999);
            UE_LOG(LogTemp, Log, TEXT("Game Status: %s"), *StatusMessage);
            
            // Generate spawn locations for game objects
            FVector GameCenter(500, 500, 100);
            for (int32 i = 0; i < 5; i++)
            {
                FVector SpawnLoc = FDependencyPluginFeatures::GetRandomSpawnLocation(GameCenter, 200.0f);
                UE_LOG(LogTemp, Log, TEXT("Enemy Spawn %d: %s"), i + 1, *SpawnLoc.ToString());
            }
            
            // Generate a system report
            FString Report = FDependencyPluginFeatures::GenerateSystemReport();
            UE_LOG(LogTemp, Log, TEXT("Game System Report:\n%s"), *Report);
        }
        else
        {
            UE_LOG(LogTemp, Error, TEXT("✗ DependencyPlugin is NOT available"));
        }
        
        UE_LOG(LogTemp, Warning, TEXT("=== Plugin Usage Demo Complete ==="));
    }
};

IMPLEMENT_PRIMARY_GAME_MODULE(FSampleProjectModule, SampleProject, "SampleProject");
